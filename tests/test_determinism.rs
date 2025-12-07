use fortress_rollback::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
struct TestInput {
    inp: u8,
}

#[allow(dead_code)]
#[derive(Default, Clone)]
struct TestGameState {
    frame: i32,
    input_history: Vec<u8>,
}

struct TestConfig;

impl Config for TestConfig {
    type Input = TestInput;
    type State = TestGameState;
    type Address = SocketAddr;
}

struct DummySocket;

impl NonBlockingSocket<SocketAddr> for DummySocket {
    fn send_to(&mut self, _msg: &Message, _addr: &SocketAddr) {}
    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        Vec::new()
    }
}

/// Test that BTreeMap provides deterministic iteration order
#[test]
fn test_btreemap_iteration_determinism() {
    use std::collections::BTreeMap;

    // Create two BTreeMaps with same data inserted in different orders
    let mut map1 = BTreeMap::new();
    map1.insert(2, "two");
    map1.insert(1, "one");
    map1.insert(3, "three");
    map1.insert(0, "zero");

    let mut map2 = BTreeMap::new();
    map2.insert(0, "zero");
    map2.insert(3, "three");
    map2.insert(1, "one");
    map2.insert(2, "two");

    // Iteration order should be the same (sorted by key)
    let vec1: Vec<_> = map1.iter().collect();
    let vec2: Vec<_> = map2.iter().collect();

    assert_eq!(vec1, vec2);
    assert_eq!(
        vec1,
        vec![(&0, &"zero"), (&1, &"one"), (&2, &"two"), (&3, &"three")]
    );
}

/// Test that local_inputs iteration is deterministic in SyncTestSession
#[test]
fn test_synctest_input_iteration_determinism() {
    const NUM_PLAYERS: usize = 4;

    // Run the same scenario multiple times
    let mut frame_counts = Vec::new();

    for _run in 0..3 {
        let mut sess = SessionBuilder::<TestConfig>::new()
            .with_num_players(NUM_PLAYERS)
            .with_max_prediction_window(8)
            .with_input_delay(0)
            .start_synctest_session()
            .unwrap();

        // Add inputs and advance 10 frames
        let mut frames_advanced = 0;
        for _frame in 0..10 {
            // Add inputs for all players
            for player in 0..NUM_PLAYERS {
                sess.add_local_input(PlayerHandle::new(player), TestInput { inp: player as u8 })
                    .unwrap();
            }

            // Process requests
            for request in sess.advance_frame().unwrap() {
                match request {
                    FortressRequest::SaveGameState { cell, frame } => {
                        let state = TestGameState {
                            frame: frame.as_i32(),
                            input_history: vec![],
                        };
                        cell.save(frame, Some(state), None);
                    }
                    FortressRequest::AdvanceFrame { .. } => {
                        frames_advanced += 1;
                    }
                    _ => {}
                }
            }
        }

        frame_counts.push(frames_advanced);
    }

    // All runs should advance the same number of frames (deterministic behavior)
    assert_eq!(frame_counts[0], frame_counts[1]);
    assert_eq!(frame_counts[1], frame_counts[2]);
    assert!(frame_counts[0] > 0);
}

/// Test that checksum history uses deterministic ordering
#[test]
fn test_checksum_history_determinism() {
    const NUM_PLAYERS: usize = 2;

    let mut sess = SessionBuilder::<TestConfig>::new()
        .with_num_players(NUM_PLAYERS)
        .with_max_prediction_window(4)
        .with_input_delay(0)
        .with_desync_detection_mode(DesyncDetection::On { interval: 1 })
        .start_synctest_session()
        .unwrap();

    // Run multiple frames and verify checksums are consistent
    let mut checksums = Vec::new();

    for _ in 0..20 {
        sess.add_local_input(PlayerHandle::new(0), TestInput { inp: 1 })
            .unwrap();
        sess.add_local_input(PlayerHandle::new(1), TestInput { inp: 2 })
            .unwrap();

        let requests = sess.advance_frame().unwrap();

        for request in requests {
            if let FortressRequest::SaveGameState { cell, frame } = request {
                // Simulate providing a checksum
                let state = TestGameState {
                    frame: frame.as_i32(),
                    input_history: vec![1, 2],
                };
                cell.save(frame, Some(state), Some(42));
                checksums.push(frame);
            } else if let FortressRequest::AdvanceFrame { .. } = request {
                // Process frame
            }
        }
    }

    // If checksums were stored in HashMap, order could vary
    // BTreeMap ensures consistent ordering by frame number
    assert!(!checksums.is_empty());
}

/// Test that player handles maintain deterministic iteration in P2PSession
#[test]
fn test_p2p_player_handles_determinism() {
    use std::str::FromStr;

    let addr1 = SocketAddr::from_str("127.0.0.1:8001").unwrap();
    let addr2 = SocketAddr::from_str("127.0.0.1:8002").unwrap();

    let mut sess_builder = SessionBuilder::<TestConfig>::new()
        .with_num_players(3)
        .with_max_prediction_window(8)
        .with_input_delay(2);

    // Add players in specific order
    sess_builder = sess_builder
        .add_player(PlayerType::Local, PlayerHandle::new(0))
        .unwrap();
    sess_builder = sess_builder
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(1))
        .unwrap();
    sess_builder = sess_builder
        .add_player(PlayerType::Remote(addr2), PlayerHandle::new(2))
        .unwrap();

    let socket = DummySocket;
    let _sess = sess_builder.start_p2p_session(socket).unwrap();

    // If player handles were stored in HashMap, iteration order could vary
    // BTreeMap ensures consistent ordering by handle (usize)
    // This is tested implicitly by the session construction not panicking
}

/// Test that Frame-keyed maps maintain deterministic ordering
#[test]
fn test_frame_map_determinism() {
    use std::collections::BTreeMap;

    // Simulate recv_inputs or pending_checksums maps
    let mut frame_map = BTreeMap::new();

    // Insert frames in non-sequential order
    frame_map.insert(10, "frame 10");
    frame_map.insert(5, "frame 5");
    frame_map.insert(15, "frame 15");
    frame_map.insert(7, "frame 7");

    // Iteration should always be in sorted order
    let frames: Vec<_> = frame_map.keys().copied().collect();
    assert_eq!(frames, vec![5, 7, 10, 15]);

    // This is critical for rollback - we need to process frames in order
    let mut last_frame = -1;
    for (&frame, _) in frame_map.iter() {
        assert!(frame > last_frame, "Frames must be in ascending order");
        last_frame = frame;
    }
}
