//! Deterministic heap-allocation contracts for documented and warmed hot paths.
//!
//! This integration test is intentionally a single test in its own process.
//! The process-local global allocator therefore cannot observe allocations from
//! another test, and every measured operation excludes construction and warm-up.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
#![forbid(unsafe_code)]

use std::alloc::System;
use std::hint::black_box;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use fortress_rollback::__internal::{ConnectionStatus, Event, PlayerInput};
use fortress_rollback::network::codec;
use fortress_rollback::{
    Config, DesyncDetection, FortressRequest, Frame, Message, MessageKind, NonBlockingSocket,
    P2PSession, PlayerHandle, SessionBuilder, SessionState, SyncTestSession,
};
use stats_alloc::{Region, Stats, StatsAlloc, INSTRUMENTED_SYSTEM};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[derive(Clone)]
struct AllocationConfig;

impl Config for AllocationConfig {
    type Input = u32;
    type State = u32;
    type Address = SocketAddr;
}

struct AllocationSocket;

impl NonBlockingSocket<SocketAddr> for AllocationSocket {
    fn send_to(&mut self, _msg: &Message, _addr: &SocketAddr) {}

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        Vec::new()
    }
}

struct SwitchableSocket {
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    sender: mpsc::Sender<(SocketAddr, Message)>,
    receiver: Mutex<mpsc::Receiver<(SocketAddr, Message)>>,
    drop_outbound: Arc<AtomicBool>,
    sends: Arc<AtomicUsize>,
}

impl NonBlockingSocket<SocketAddr> for SwitchableSocket {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        if *addr != self.peer_addr {
            return;
        }
        self.sends.fetch_add(1, Ordering::Relaxed);
        if !self.drop_outbound.load(Ordering::Relaxed) {
            let _result = self.sender.send((self.local_addr, msg.clone()));
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        self.receiver
            .lock()
            .expect("allocation socket mutex poisoned")
            .try_iter()
            .collect()
    }
}

fn switchable_socket_pair() -> (
    SwitchableSocket,
    SwitchableSocket,
    Arc<AtomicBool>,
    Arc<AtomicUsize>,
) {
    let addr_a = SocketAddr::from(([127, 0, 0, 1], 31_001));
    let addr_b = SocketAddr::from(([127, 0, 0, 1], 31_002));
    let (to_a, from_b) = mpsc::channel();
    let (to_b, from_a) = mpsc::channel();
    let drop_a = Arc::new(AtomicBool::new(false));
    let sends_a = Arc::new(AtomicUsize::new(0));

    (
        SwitchableSocket {
            local_addr: addr_a,
            peer_addr: addr_b,
            sender: to_b,
            receiver: Mutex::new(from_b),
            drop_outbound: Arc::clone(&drop_a),
            sends: Arc::clone(&sends_a),
        },
        SwitchableSocket {
            local_addr: addr_b,
            peer_addr: addr_a,
            sender: to_a,
            receiver: Mutex::new(from_a),
            drop_outbound: Arc::new(AtomicBool::new(false)),
            sends: Arc::new(AtomicUsize::new(0)),
        },
        drop_a,
        sends_a,
    )
}

fn measure<T>(operation: impl FnOnce() -> T) -> (T, Stats) {
    let region = Region::new(GLOBAL);
    let result = operation();
    let stats = region.change();
    (result, stats)
}

#[track_caller]
fn assert_zero_allocations(label: &str, stats: Stats) {
    assert_eq!(stats.allocations, 0, "{label} allocation stats: {stats:?}");
    assert_eq!(
        stats.reallocations, 0,
        "{label} allocation stats: {stats:?}"
    );
    assert_eq!(
        stats.bytes_allocated, 0,
        "{label} allocation stats: {stats:?}"
    );
    assert_eq!(
        stats.bytes_reallocated, 0,
        "{label} allocation stats: {stats:?}"
    );
}

fn allocated_and_grown_bytes(stats: Stats) -> usize {
    let reallocation_growth = usize::try_from(stats.bytes_reallocated.max(0))
        .expect("nonnegative reallocation growth fits usize");
    stats.bytes_allocated.saturating_add(reallocation_growth)
}

#[track_caller]
fn assert_allocation_ceiling(
    label: &str,
    stats: Stats,
    maximum_operations: usize,
    maximum_bytes: usize,
) {
    let operations = stats.allocations.saturating_add(stats.reallocations);
    let allocated_bytes = allocated_and_grown_bytes(stats);
    assert!(
        operations <= maximum_operations,
        "{label} used {operations} allocation operations (ceiling {maximum_operations}): {stats:?}"
    );
    assert!(
        allocated_bytes <= maximum_bytes,
        "{label} allocated or grew by {allocated_bytes} bytes (ceiling {maximum_bytes}): {stats:?}"
    );
}

fn warmed_synctest_frame_stats(num_players: usize) -> [Stats; 3] {
    let mut session: SyncTestSession<AllocationConfig> = SessionBuilder::new()
        .with_num_players(num_players)
        .expect("configure allocation-contract player count")
        .with_check_distance(0)
        .start_synctest_session()
        .expect("construct allocation-contract sync test");

    for player in 0..num_players {
        session
            .add_local_input(
                PlayerHandle::new(player),
                u32::try_from(player).expect("allocation-contract handle fits u32"),
            )
            .expect("add warm-up input");
    }
    fulfill_save_requests(session.advance_frame().expect("advance warm-up frame"));

    std::array::from_fn(|_| {
        let (requests, stats) = measure(|| {
            for player in 0..num_players {
                session
                    .add_local_input(
                        PlayerHandle::new(player),
                        u32::try_from(player).expect("allocation-contract handle fits u32"),
                    )
                    .expect("add measured input");
            }
            black_box(session.advance_frame().expect("advance measured frame"))
        });
        fulfill_save_requests(requests);
        stats
    })
}

fn warmed_all_local_p2p_frame_stats(num_players: usize) -> [Stats; 3] {
    let mut builder = SessionBuilder::new()
        .with_num_players(num_players)
        .expect("configure allocation-contract P2P player count")
        .with_desync_detection_mode(DesyncDetection::Off);
    for player in 0..num_players {
        builder = builder
            .add_local_player(player)
            .expect("add allocation-contract P2P local player");
    }
    let mut session: P2PSession<AllocationConfig> = builder
        .start_p2p_session(AllocationSocket)
        .expect("construct allocation-contract P2P session");

    for player in 0..num_players {
        session
            .add_local_input(
                PlayerHandle::new(player),
                u32::try_from(player).expect("allocation-contract handle fits u32"),
            )
            .expect("add P2P warm-up input");
    }
    fulfill_save_requests(session.advance_frame().expect("advance P2P warm-up frame"));

    std::array::from_fn(|_| {
        let (requests, stats) = measure(|| {
            for player in 0..num_players {
                session
                    .add_local_input(
                        PlayerHandle::new(player),
                        u32::try_from(player).expect("allocation-contract handle fits u32"),
                    )
                    .expect("add measured P2P input");
            }
            black_box(session.advance_frame().expect("advance measured P2P frame"))
        });
        fulfill_save_requests(requests);
        stats
    })
}

fn fulfill_save_requests(requests: fortress_rollback::RequestVec<AllocationConfig>) {
    for request in requests {
        match request {
            FortressRequest::SaveGameState { cell, frame } => {
                cell.save(frame, Some(0), Some(0));
            },
            FortressRequest::AdvanceFrame { .. } => {},
            FortressRequest::LoadGameState { frame, .. } => {
                panic!("allocation fixture unexpectedly requested rollback to {frame}");
            },
        }
    }
}

fn warmed_networked_p2p_send_stats(num_players: usize) -> Stats {
    let (socket_a, socket_b, drop_a, sends_a) = switchable_socket_pair();
    let addr_a = socket_a.local_addr;
    let addr_b = socket_b.local_addr;
    let remote_handle = PlayerHandle::new(num_players.saturating_sub(1));

    let mut builder_a = SessionBuilder::new()
        .with_num_players(num_players)
        .expect("configure networked allocation-contract player count")
        .with_desync_detection_mode(DesyncDetection::Off);
    let mut builder_b = SessionBuilder::new()
        .with_num_players(num_players)
        .expect("configure networked allocation-contract player count")
        .with_desync_detection_mode(DesyncDetection::Off);
    for player in 0..num_players {
        if player + 1 == num_players {
            builder_a = builder_a
                .add_remote_player(player, addr_b)
                .expect("add allocation-contract remote player to A");
            builder_b = builder_b
                .add_local_player(player)
                .expect("add allocation-contract local player to B");
        } else {
            builder_a = builder_a
                .add_local_player(player)
                .expect("add allocation-contract local player to A");
            builder_b = builder_b
                .add_remote_player(player, addr_a)
                .expect("add allocation-contract remote player to B");
        }
    }
    let mut session_a: P2PSession<AllocationConfig> = builder_a
        .start_p2p_session(socket_a)
        .expect("construct networked allocation-contract session A");
    let mut session_b: P2PSession<AllocationConfig> = builder_b
        .start_p2p_session(socket_b)
        .expect("construct networked allocation-contract session B");

    for _ in 0..64 {
        session_a.poll_remote_clients();
        session_b.poll_remote_clients();
        if session_a.current_state() == SessionState::Running
            && session_b.current_state() == SessionState::Running
        {
            break;
        }
    }
    assert_eq!(session_a.current_state(), SessionState::Running);
    assert_eq!(session_b.current_state(), SessionState::Running);

    for warm_up_frame in 0..2 {
        for player in 0..num_players.saturating_sub(1) {
            session_a
                .add_local_input(PlayerHandle::new(player), 0)
                .expect("add networked warm-up input to A");
        }
        session_b
            .add_local_input(remote_handle, 0)
            .expect("add networked warm-up input to B");
        fulfill_save_requests(
            session_a
                .advance_frame()
                .expect("advance networked warm-up frame on A"),
        );
        fulfill_save_requests(
            session_b
                .advance_frame()
                .expect("advance networked warm-up frame on B"),
        );
        for _ in 0..4 {
            session_a.poll_remote_clients();
            session_b.poll_remote_clients();
        }
        assert_eq!(
            session_a
                .peer_metrics(remote_handle)
                .expect("read A peer metrics")
                .pending_output_len,
            0,
            "warm-up input {warm_up_frame} must be acknowledged before measuring"
        );
    }

    drop_a.store(true, Ordering::Relaxed);
    let sends_before = sends_a.load(Ordering::Relaxed);
    let metrics_before = session_a
        .peer_metrics(remote_handle)
        .expect("read A peer metrics before measurement");
    let (requests, stats) = measure(|| {
        for player in 0..num_players.saturating_sub(1) {
            session_a
                .add_local_input(PlayerHandle::new(player), 0)
                .expect("add measured networked input to A");
        }
        black_box(
            session_a
                .advance_frame()
                .expect("advance measured networked frame on A"),
        )
    });
    assert_eq!(
        requests
            .iter()
            .filter(|request| matches!(request, FortressRequest::AdvanceFrame { .. }))
            .count(),
        1,
        "the measured frame must request exactly one application advance"
    );
    assert!(
        requests
            .iter()
            .all(|request| !matches!(request, FortressRequest::LoadGameState { .. })),
        "the measured steady-state frame must not include rollback work"
    );
    black_box(requests);
    assert_eq!(
        sends_a.load(Ordering::Relaxed),
        sends_before + 1,
        "measured frame must exercise one endpoint send"
    );
    let metrics_after = session_a
        .peer_metrics(remote_handle)
        .expect("read A peer metrics after measurement");
    assert_eq!(
        metrics_after.messages_sent_by_kind.get(MessageKind::Input),
        metrics_before.messages_sent_by_kind.get(MessageKind::Input) + 1,
        "the measured endpoint message must be an Input packet"
    );
    assert_eq!(
        metrics_after.input_bytes_pre_compression,
        metrics_before.input_bytes_pre_compression
            + u64::try_from(num_players.saturating_sub(1) * size_of::<u32>())
                .expect("bounded input bytes fit u64"),
        "the measured Input packet must include every local player's bytes"
    );
    assert!(
        metrics_after.input_bytes_post_compression > metrics_before.input_bytes_post_compression,
        "the measured Input packet must exercise delta/RLE compression"
    );
    assert_eq!(
        metrics_after.pending_output_len, 1,
        "the non-allocating sink must leave exactly one bounded frame awaiting acknowledgement"
    );
    stats
}

fn warmed_spectator_poll_stats(num_hosts: usize) -> Stats {
    let hosts: Vec<_> = (0..num_hosts)
        .map(|index| {
            let port = 32_001_u16
                .checked_add(u16::try_from(index).expect("allocation host index fits u16"))
                .expect("allocation host port stays in range");
            SocketAddr::from(([127, 0, 0, 1], port))
        })
        .collect();
    let mut session = SessionBuilder::<AllocationConfig>::new()
        .with_num_players(2)
        .expect("configure allocation-contract spectator player count")
        .start_spectator_session_multi(&hosts, AllocationSocket)
        .expect("construct allocation-contract spectator session");

    session.poll_remote_clients();
    let ((), stats) = measure(|| {
        session.poll_remote_clients();
        black_box(())
    });
    stats
}

fn input_event_snapshot_clone_stats(num_players: usize) -> Stats {
    let event: Event<AllocationConfig> = Event::Input {
        input: PlayerInput::new(Frame::new(0), 0),
        player: PlayerHandle::new(0),
        peer_connect_status: vec![ConnectionStatus::default(); num_players].into(),
    };
    let mut clones = Vec::with_capacity(num_players);
    let ((), stats) = measure(|| {
        for _ in 0..num_players {
            clones.push(black_box(&event).clone());
        }
    });
    black_box(clones);
    stats
}

/// Allocation contracts are measured together so the instrumented allocator
/// cannot observe concurrently running tests in this integration-test process.
#[test]
fn warmed_hot_paths_obey_allocation_contracts() {
    // Counter sensitivity: this must detect one known allocation or every
    // zero-allocation assertion below would be capable of passing vacuously.
    let (allocation, control) = measure(|| vec![0_u8; 4_096]);
    black_box(&allocation);
    assert_eq!(
        control.allocations, 1,
        "known-allocation control: {control:?}"
    );
    assert_eq!(
        control.reallocations, 0,
        "known-allocation control: {control:?}"
    );
    assert_eq!(
        control.bytes_allocated, 4_096,
        "known-allocation control: {control:?}"
    );
    drop(allocation);

    // The zero-allocation helper deliberately permits deallocation: its
    // contract is about newly allocated memory, not every allocator event.
    let deallocation = vec![0_u8; 4_096];
    let ((), deallocation_control) = measure(|| drop(deallocation));
    assert_eq!(
        deallocation_control.deallocations, 1,
        "known-deallocation control: {deallocation_control:?}"
    );
    assert_zero_allocations("known-deallocation control", deallocation_control);

    // Synthetic counter sensitivity ensures positive reallocation growth is
    // charged to the byte ceiling even when no fresh allocation is recorded.
    let reallocation_control = Stats {
        reallocations: 1,
        bytes_reallocated: 4_096,
        ..Stats::default()
    };
    assert_eq!(
        allocated_and_grown_bytes(reallocation_control),
        4_096,
        "known-reallocation-growth control: {reallocation_control:?}"
    );
    assert_allocation_ceiling(
        "known-reallocation-growth control",
        reallocation_control,
        1,
        4_096,
    );

    // `encode_into` publicly documents that a caller-provided buffer avoids
    // allocation on the successful path.
    let mut buffer = [0_u8; 64];
    let (encoded_len, encode_stats) =
        measure(|| codec::encode_into(black_box(&42_u32), black_box(&mut buffer)));
    assert_eq!(encoded_len.expect("encode into fixed buffer"), 4);
    assert_zero_allocations("codec::encode_into", encode_stats);

    // Handle iterators publicly document zero allocation. The collecting
    // convenience API is allowed one bounded spill above HandleVec's inline
    // capacity of eight.
    let session: SyncTestSession<AllocationConfig> = SessionBuilder::new()
        .with_num_players(16)
        .expect("configure sixteen-player allocation contract")
        .start_synctest_session()
        .expect("construct sixteen-player allocation contract");
    let (handle_sum, iterator_stats) = measure(|| {
        black_box(&session)
            .local_player_handles_iter()
            .map(PlayerHandle::as_usize)
            .sum::<usize>()
    });
    assert_eq!(handle_sum, 120);
    assert_zero_allocations("local_player_handles_iter", iterator_stats);

    let (handles, collecting_stats) = measure(|| session.local_player_handles());
    assert_eq!(handles.len(), 16);
    assert_allocation_ceiling("sixteen-player HandleVec spill", collecting_stats, 1, 256);
    drop(handles);

    // Warmed frame input staging reuses constructor-owned storage at every
    // scale. N=2 and N=4 therefore touch no allocator; N=16 retains one exact
    // 128-byte spill for the returned 16-player InputVec above inline capacity.
    for (players, operation_ceiling, byte_ceiling) in [(2, 0, 0), (4, 0, 0), (16, 1, 128)] {
        let repeated = warmed_synctest_frame_stats(players);
        let first = repeated[0];
        for (repetition, stats) in repeated.iter().enumerate().skip(1) {
            assert_eq!(
                *stats, first,
                "warmed {players}-player sync-test frame repetition {repetition}"
            );
        }
        assert_allocation_ceiling(
            &format!("warmed {players}-player sync-test frame"),
            first,
            operation_ceiling,
            byte_ceiling,
        );
    }

    for (players, operation_ceiling, byte_ceiling) in [(2, 0, 0), (4, 0, 0), (16, 1, 128)] {
        let repeated = warmed_all_local_p2p_frame_stats(players);
        let first = repeated[0];
        for (repetition, stats) in repeated.iter().enumerate().skip(1) {
            assert_eq!(
                *stats, first,
                "warmed {players}-player all-local P2P frame repetition {repetition}"
            );
        }
        assert_allocation_ceiling(
            &format!("warmed {players}-player all-local P2P frame"),
            first,
            operation_ceiling,
            byte_ceiling,
        );
    }

    // This real synchronized one-endpoint path includes Fortress's internal
    // receive/control poll plus input serialization, delta/RLE compression,
    // status gossip, and request construction. The switchable socket counts
    // submission but deliberately excludes adapter buffering and cloning. Its
    // allocation contract is bounded rather than zero: one input buffer stays
    // retained until acknowledgment, and compression/status temporaries are
    // released after submission. The byte ceilings also prevent the empty
    // graceful-drop poll from restoring its former 28 KiB reservation.
    for (players, operation_ceiling, byte_ceiling) in [(2, 4, 128), (4, 4, 128), (16, 5, 512)] {
        let first = warmed_networked_p2p_send_stats(players);
        for repetition in 1..3 {
            assert_eq!(
                warmed_networked_p2p_send_stats(players),
                first,
                "warmed {players}-player networked P2P send repetition {repetition}"
            );
        }
        assert_allocation_ceiling(
            &format!("warmed {players}-player networked P2P send"),
            first,
            operation_ceiling,
            byte_ceiling,
        );
    }

    // An input-idle spectator still polls every render/network tick. Once its
    // host-indexed scratch space is warmed, that no-message/no-event path must
    // retain and reuse the bounded storage instead of allocating per poll.
    for hosts in [1, 4] {
        let first = warmed_spectator_poll_stats(hosts);
        for repetition in 1..3 {
            assert_eq!(
                warmed_spectator_poll_stats(hosts),
                first,
                "warmed {hosts}-host spectator poll repetition {repetition}"
            );
        }
        assert_zero_allocations(&format!("warmed {hosts}-host spectator poll"), first);
    }

    // Every player event decoded from one Input frame observes the same
    // immutable connection-status snapshot. Cloning that internal event must
    // share the snapshot instead of duplicating its player-count-sized buffer.
    for players in [2, 4, 16] {
        assert_zero_allocations(
            &format!("{players}-player input-event snapshot clones"),
            input_event_snapshot_clone_stats(players),
        );
    }
}
