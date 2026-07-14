//! Runtime-to-TLA+ handshake trace driver over the deterministic SimNet fabric.

use crate::common::sim_net::{LinkPolicy, SimNet};
use crate::common::stubs::StubConfig;
use crate::common::test_clock::TestClock;
use fortress_rollback::__internal::{
    HandshakeReplyDisposition, HandshakeRequestDisposition, HandshakeRequestIdDisposition,
    ProtocolState,
};
use fortress_rollback::__internal::{HandshakeTraceAction, HandshakeTraceEvent};
use fortress_rollback::{
    IncompatibleSessionReason, Message, MessageKind, NonBlockingSocket, P2PSession, PlayerHandle,
    PlayerType, ProtocolConfig, SessionBuilder, SyncConfig,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use web_time::Duration;

const TRACE_CAPACITY: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OrderedEvent {
    peer: usize,
    event: HandshakeTraceEvent,
}

struct DuplicateFirstReplySocket {
    inner: crate::common::sim_net::SimSocket<Message>,
    duplicated: bool,
}

impl NonBlockingSocket<SocketAddr> for DuplicateFirstReplySocket {
    fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
        self.inner.send_payload(*addr, msg.clone());
        if !self.duplicated
            && fortress_rollback::__internal::message_metadata(msg).1 == MessageKind::SyncReply
        {
            self.inner.send_payload(*addr, msg.clone());
            self.duplicated = true;
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
        self.inner.recv_payloads()
    }
}

fn peer_addr(peer: usize) -> SocketAddr {
    let port = 46_000_u16.saturating_add(u16::try_from(peer).unwrap_or(u16::MAX));
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
}

fn build_session(
    peer: usize,
    num_players: usize,
    socket: impl NonBlockingSocket<SocketAddr> + 'static,
    clock: &TestClock,
    sync_config: SyncConfig,
) -> P2PSession<StubConfig> {
    let mut builder = SessionBuilder::<StubConfig>::new()
        .with_num_players(num_players)
        .expect("trace player count is valid")
        .with_sync_config(sync_config)
        .with_protocol_config(ProtocolConfig {
            clock: Some(clock.as_protocol_clock()),
            protocol_rng_seed: Some(0x51_0000_u64.saturating_add(peer as u64)),
            ..ProtocolConfig::default()
        })
        .with_handshake_trace_capacity(TRACE_CAPACITY)
        .expect("trace capacity is within the semantic cap");
    for handle in 0..num_players {
        let player_type = if handle == peer || (num_players == 3 && handle == 2) {
            PlayerType::Local
        } else {
            PlayerType::Remote(peer_addr(handle))
        };
        builder = builder
            .add_player(player_type, PlayerHandle::new(handle))
            .expect("trace player registration is valid");
    }
    builder
        .start_p2p_session(socket)
        .expect("trace session starts")
}

fn collect_new(
    peer: usize,
    session: &P2PSession<StubConfig>,
    remote_handle: usize,
    cursor: &mut usize,
    ordered: &mut Vec<OrderedEvent>,
) {
    let trace = session
        .handshake_trace(PlayerHandle::new(remote_handle))
        .expect("trace endpoint exists")
        .expect("trace recorder was enabled")
        .expect("trace recorder did not overflow");
    assert!(*cursor <= trace.len(), "trace cursor must remain monotonic");
    ordered.extend(
        trace[*cursor..]
            .iter()
            .copied()
            .map(|event| OrderedEvent { peer, event }),
    );
    *cursor = trace.len();
}

fn matching_runtime_events() -> Vec<OrderedEvent> {
    let clock = TestClock::new();
    let net = SimNet::<Message>::new(0x51_51, clock.as_protocol_clock());
    let addr0 = peer_addr(0);
    let addr1 = peer_addr(1);
    let socket0 = net.attach(addr0);
    let socket1 = net.attach(addr1);
    let sync_config = SyncConfig {
        num_sync_packets: 2,
        sync_retry_interval: Duration::from_millis(100),
        sync_timeout: Some(Duration::from_secs(1)),
        ..SyncConfig::default()
    };

    let mut peer0 = build_session(0, 2, socket0, &clock, sync_config);
    let mut peer1 = build_session(
        1,
        2,
        DuplicateFirstReplySocket {
            inner: socket1,
            duplicated: false,
        },
        &clock,
        sync_config,
    );
    let mut cursors = [0_usize; 2];
    let mut ordered = Vec::new();
    collect_new(0, &peer0, 1, &mut cursors[0], &mut ordered);
    collect_new(1, &peer1, 0, &mut cursors[1], &mut ordered);

    // Flush peer 1's initial request, then let peer 0 answer it and send its own
    // initial request. Peer 1's next poll handles that request; its socket
    // wrapper duplicates exactly reply A without duplicating unrelated requests.
    peer1.poll_remote_clients();
    collect_new(1, &peer1, 0, &mut cursors[1], &mut ordered);
    peer0.poll_remote_clients();
    collect_new(0, &peer0, 1, &mut cursors[0], &mut ordered);
    peer1.poll_remote_clients();
    collect_new(1, &peer1, 0, &mut cursors[1], &mut ordered);

    peer0.poll_remote_clients();
    collect_new(0, &peer0, 1, &mut cursors[0], &mut ordered);

    // Delay peer 1's reply to the second peer-0 request, then permit the next
    // retry's reply through immediately. This makes fresh request C complete
    // before still-outstanding B without changing either endpoint's RNG stream.
    net.set_link(
        addr1,
        addr0,
        LinkPolicy {
            base_delay: Duration::from_secs(1),
            ..LinkPolicy::clean()
        },
    );
    peer1.poll_remote_clients();
    collect_new(1, &peer1, 0, &mut cursors[1], &mut ordered);
    net.set_link(addr1, addr0, LinkPolicy::clean());

    clock.advance(Duration::from_millis(101));
    peer0.poll_remote_clients();
    collect_new(0, &peer0, 1, &mut cursors[0], &mut ordered);
    // The retry C was queued by the preceding poll; flush it into SimNet.
    peer0.poll_remote_clients();
    collect_new(0, &peer0, 1, &mut cursors[0], &mut ordered);
    peer1.poll_remote_clients();
    collect_new(1, &peer1, 0, &mut cursors[1], &mut ordered);
    // Flush peer 1's clean reply C.
    peer1.poll_remote_clients();
    collect_new(1, &peer1, 0, &mut cursors[1], &mut ordered);
    peer0.poll_remote_clients();
    collect_new(0, &peer0, 1, &mut cursors[0], &mut ordered);
    peer1.poll_remote_clients();
    collect_new(1, &peer1, 0, &mut cursors[1], &mut ordered);

    ordered
}

fn mismatch_runtime_events() -> Vec<OrderedEvent> {
    let clock = TestClock::new();
    let net = SimNet::<Message>::new(0x52_52, clock.as_protocol_clock());
    let sync_config = SyncConfig {
        num_sync_packets: 2,
        sync_retry_interval: Duration::from_millis(100),
        sync_timeout: Some(Duration::from_secs(1)),
        ..SyncConfig::default()
    };
    let mut peer0 = build_session(0, 2, net.attach(peer_addr(0)), &clock, sync_config);
    let mut peer1 = build_session(1, 3, net.attach(peer_addr(1)), &clock, sync_config);
    let mut cursors = [0_usize; 2];
    let mut ordered = Vec::new();
    collect_new(0, &peer0, 1, &mut cursors[0], &mut ordered);
    collect_new(1, &peer1, 0, &mut cursors[1], &mut ordered);

    peer1.poll_remote_clients();
    collect_new(1, &peer1, 0, &mut cursors[1], &mut ordered);
    peer0.poll_remote_clients();
    collect_new(0, &peer0, 1, &mut cursors[0], &mut ordered);
    peer1.poll_remote_clients();
    collect_new(1, &peer1, 0, &mut cursors[1], &mut ordered);
    peer0.poll_remote_clients();
    collect_new(0, &peer0, 1, &mut cursors[0], &mut ordered);

    ordered
}

fn timeout_runtime_events() -> Vec<OrderedEvent> {
    let clock = TestClock::new();
    let net = SimNet::<Message>::new(0x53_53, clock.as_protocol_clock());
    let addr0 = peer_addr(0);
    let addr1 = peer_addr(1);
    net.set_blocked(addr0, addr1, true);
    net.set_blocked(addr1, addr0, true);
    let sync_config = SyncConfig {
        num_sync_packets: 2,
        sync_retry_interval: Duration::from_millis(100),
        sync_timeout: Some(Duration::from_millis(500)),
        ..SyncConfig::default()
    };
    let mut peer0 = build_session(0, 2, net.attach(addr0), &clock, sync_config);
    let peer1 = build_session(1, 2, net.attach(addr1), &clock, sync_config);
    let mut cursors = [0_usize; 2];
    let mut ordered = Vec::new();
    collect_new(0, &peer0, 1, &mut cursors[0], &mut ordered);
    collect_new(1, &peer1, 0, &mut cursors[1], &mut ordered);

    clock.advance(Duration::from_millis(501));
    peer0.poll_remote_clients();
    collect_new(0, &peer0, 1, &mut cursors[0], &mut ordered);

    ordered
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
struct ModelConfig {
    #[serde(rename = "numPlayers")]
    num_players: u16,
    #[serde(rename = "inputWidth")]
    input_width: u16,
}

impl From<fortress_rollback::__internal::HandshakeTraceConfig> for ModelConfig {
    fn from(config: fortress_rollback::__internal::HandshakeTraceConfig) -> Self {
        Self {
            num_players: config.num_players,
            input_width: config.input_bytes_per_player,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
struct ModelMessage {
    kind: &'static str,
    from: &'static str,
    to: &'static str,
    token: u32,
    config: ModelConfig,
}

#[derive(Clone, Debug)]
struct ModelState {
    phase: [&'static str; 2],
    local_config: [ModelConfig; 2],
    learned_config: [Option<ModelConfig>; 2],
    learned_from: [Option<&'static str>; 2],
    sync_remaining: [u32; 2],
    accepted_tokens: [Vec<u32>; 2],
    next_token: [u32; 2],
    timeout_ticks: [u32; 2],
    timeout_events: [u32; 2],
    incompatible_events: [u32; 2],
    reason_field: [&'static str; 2],
    reason_ours: [Option<u32>; 2],
    reason_theirs: [Option<u32>; 2],
    network: Vec<ModelMessage>,
}

impl ModelState {
    fn peer_name(peer: usize) -> &'static str {
        if peer == 0 {
            "p1"
        } else {
            "p2"
        }
    }

    fn peer_map<T: serde::Serialize>(values: &[T; 2]) -> serde_json::Value {
        serde_json::json!({"p1": &values[0], "p2": &values[1]})
    }

    fn complete_json(&self) -> serde_json::Value {
        serde_json::json!({
            "phase": Self::peer_map(&self.phase),
            "localConfig": Self::peer_map(&self.local_config),
            "learnedConfig": Self::peer_map(&self.learned_config),
            "learnedFrom": Self::peer_map(&self.learned_from),
            "syncRemaining": Self::peer_map(&self.sync_remaining),
            "acceptedTokens": Self::peer_map(&self.accepted_tokens),
            "nextToken": Self::peer_map(&self.next_token),
            "timeoutTicks": Self::peer_map(&self.timeout_ticks),
            "timeoutEventCount": Self::peer_map(&self.timeout_events),
            "incompatibleEventCount": Self::peer_map(&self.incompatible_events),
            "reasonField": Self::peer_map(&self.reason_field),
            "reasonOurs": Self::peer_map(&self.reason_ours),
            "reasonTheirs": Self::peer_map(&self.reason_theirs),
            "network": &self.network,
        })
    }

    fn updates_json(&self, action: &str) -> serde_json::Value {
        let complete = self.complete_json();
        let names: &[&str] = match action {
            "SendSyncRequest" => &["nextToken", "network"],
            "HandleSyncRequest" => &[
                "phase",
                "learnedConfig",
                "learnedFrom",
                "incompatibleEventCount",
                "reasonField",
                "reasonOurs",
                "reasonTheirs",
                "network",
            ],
            "HandleSyncReply" => &[
                "phase",
                "learnedConfig",
                "learnedFrom",
                "syncRemaining",
                "acceptedTokens",
                "incompatibleEventCount",
                "reasonField",
                "reasonOurs",
                "reasonTheirs",
                "network",
            ],
            "DuplicateMessage" => &["network"],
            "TickTimeout" => &["timeoutTicks"],
            "ReportSyncTimeout" => &["timeoutEventCount"],
            _ => panic!("unsupported trace action {action}"),
        };
        let object = complete.as_object().expect("complete state is an object");
        serde_json::Value::Object(
            names
                .iter()
                .map(|name| {
                    (
                        (*name).to_owned(),
                        object.get(*name).expect("state variable exists").clone(),
                    )
                })
                .collect(),
        )
    }

    fn apply_runtime_snapshot(&mut self, ordered: &OrderedEvent, expected_outstanding: usize) {
        let peer = ordered.peer;
        assert_eq!(
            ModelConfig::from(ordered.event.local_config),
            self.local_config[peer],
            "runtime local configuration must remain stable"
        );
        assert_eq!(
            ordered.event.outstanding_request_count, expected_outstanding,
            "normalizer outstanding-ID reconstruction must match runtime"
        );
        self.sync_remaining[peer] = ordered.event.sync_remaining_roundtrips;
        self.timeout_events[peer] = u32::from(ordered.event.timeout_event_sent);
        match ordered.event.incompatibility {
            Some(IncompatibleSessionReason::NumPlayers { ours, theirs }) => {
                self.phase[peer] = "Failed";
                self.incompatible_events[peer] = 1;
                self.reason_field[peer] = "NumPlayers";
                self.reason_ours[peer] = Some(u32::from(ours));
                self.reason_theirs[peer] = Some(u32::from(theirs));
            },
            Some(IncompatibleSessionReason::InputWidth { ours, theirs }) => {
                self.phase[peer] = "Failed";
                self.incompatible_events[peer] = 1;
                self.reason_field[peer] = "InputWidth";
                self.reason_ours[peer] = Some(u32::from(ours));
                self.reason_theirs[peer] = Some(u32::from(theirs));
            },
            Some(other) => panic!("runtime mismatch is outside the modeled projection: {other:?}"),
            None => {
                self.phase[peer] = match ordered.event.state {
                    ProtocolState::Synchronizing => "Syncing",
                    ProtocolState::Running => "Synced",
                    other => panic!("runtime phase is outside the handshake model: {other:?}"),
                };
                self.incompatible_events[peer] = 0;
                self.reason_field[peer] = "None";
                self.reason_ours[peer] = None;
                self.reason_theirs[peer] = None;
            },
        }
        assert_eq!(
            self.accepted_tokens[peer].len() as u32,
            2_u32.saturating_sub(self.sync_remaining[peer]),
            "runtime remaining-roundtrip count must match accepted identities"
        );
    }
}

struct NormalizedTrace {
    records: Vec<serde_json::Value>,
    duplicate_reply_step: Option<usize>,
}

fn push_action(
    records: &mut Vec<serde_json::Value>,
    state: &ModelState,
    action: &'static str,
    peer: usize,
) {
    records.push(serde_json::json!({
        "step": records.len() - 1,
        "action": action,
        "peer": ModelState::peer_name(peer),
        "updates": state.updates_json(action),
    }));
}

fn normalize_runtime_trace(
    name: &'static str,
    description: &'static str,
    events: &[OrderedEvent],
) -> NormalizedTrace {
    let mut local_configs = [None; 2];
    let mut initial_remaining = [None; 2];
    for ordered in events {
        if matches!(ordered.event.action, HandshakeTraceAction::Activated) {
            local_configs[ordered.peer] = Some(ordered.event.local_config.into());
        }
        if matches!(
            ordered.event.action,
            HandshakeTraceAction::BeginSynchronization
        ) {
            assert_eq!(ordered.event.state, ProtocolState::Synchronizing);
            assert_eq!(ordered.event.incompatibility, None);
            assert!(!ordered.event.timeout_event_sent);
            initial_remaining[ordered.peer] = Some(ordered.event.sync_remaining_roundtrips);
        }
    }
    let mut state = ModelState {
        phase: ["Syncing", "Syncing"],
        local_config: [
            local_configs[0].expect("peer 1 activation is recorded"),
            local_configs[1].expect("peer 2 activation is recorded"),
        ],
        learned_config: [None, None],
        learned_from: [None, None],
        sync_remaining: [
            initial_remaining[0].expect("peer 1 begin transition is recorded"),
            initial_remaining[1].expect("peer 2 begin transition is recorded"),
        ],
        accepted_tokens: [Vec::new(), Vec::new()],
        next_token: [1, 1],
        timeout_ticks: [0, 0],
        timeout_events: [0, 0],
        incompatible_events: [0, 0],
        reason_field: ["None", "None"],
        reason_ours: [None, None],
        reason_theirs: [None, None],
        network: Vec::new(),
    };
    let mut records = vec![serde_json::json!({
        "schema": 1,
        "trace": name,
        "expect": "accept",
        "description": description,
    })];
    records.push(serde_json::json!({
        "step": 0,
        "action": "Init",
        "state": state.complete_json(),
    }));

    let duplicate_replies: std::collections::BTreeSet<(usize, u32)> = events
        .iter()
        .filter_map(|ordered| match ordered.event.action {
            HandshakeTraceAction::HandleReply {
                request_id,
                disposition: HandshakeReplyDisposition::Duplicate,
            } => Some((ordered.peer, request_id)),
            _ => None,
        })
        .collect();
    let mut duplicated = std::collections::BTreeSet::new();
    let mut raw_ids = [
        std::collections::BTreeMap::<u32, u32>::new(),
        std::collections::BTreeMap::<u32, u32>::new(),
    ];
    let mut outstanding_ids = [
        std::collections::BTreeSet::<u32>::new(),
        std::collections::BTreeSet::<u32>::new(),
    ];
    let mut duplicate_reply_step = None;

    for ordered in events {
        let peer = ordered.peer;
        let other = 1 - peer;
        match ordered.event.action {
            HandshakeTraceAction::Activated | HandshakeTraceAction::BeginSynchronization => {},
            HandshakeTraceAction::SendRequest {
                request_id,
                disposition,
            } => {
                assert_eq!(disposition, HandshakeRequestIdDisposition::Fresh);
                let token = state.next_token[peer];
                assert!(
                    token <= 3,
                    "runtime request namespace exceeds the model bound"
                );
                assert_eq!(raw_ids[peer].insert(request_id, token), None);
                assert!(outstanding_ids[peer].insert(request_id));
                state.next_token[peer] += 1;
                state.network.push(ModelMessage {
                    kind: "SyncRequest",
                    from: ModelState::peer_name(peer),
                    to: ModelState::peer_name(other),
                    token,
                    config: state.local_config[peer],
                });
                state.apply_runtime_snapshot(ordered, outstanding_ids[peer].len());
                push_action(&mut records, &state, "SendSyncRequest", peer);
            },
            HandshakeTraceAction::HandleRequest {
                request_id,
                disposition,
            } => {
                let token = *raw_ids[other]
                    .get(&request_id)
                    .expect("handled request was emitted by the other endpoint");
                let index = state
                    .network
                    .iter()
                    .position(|message| {
                        message.kind == "SyncRequest"
                            && message.to == ModelState::peer_name(peer)
                            && message.token == token
                    })
                    .expect("handled request is present in the modeled network");
                let request = state.network.remove(index);
                state.network.push(ModelMessage {
                    kind: "SyncReply",
                    from: ModelState::peer_name(peer),
                    to: ModelState::peer_name(other),
                    token,
                    config: state.local_config[peer],
                });
                let remote_config: ModelConfig = ordered
                    .event
                    .remote_config
                    .expect("handled request records its remote config")
                    .into();
                assert_eq!(remote_config, request.config);
                state.learned_config[peer] = Some(remote_config);
                state.learned_from[peer] = Some(ModelState::peer_name(other));
                let expected_disposition = if state.phase[peer] == "Failed" {
                    HandshakeRequestDisposition::AlreadyIncompatible
                } else if state.phase[peer] == "Synced" {
                    HandshakeRequestDisposition::AnsweredOnly
                } else if remote_config != state.local_config[peer] {
                    HandshakeRequestDisposition::Incompatible
                } else {
                    HandshakeRequestDisposition::Observed
                };
                assert_eq!(disposition, expected_disposition);
                state.apply_runtime_snapshot(ordered, outstanding_ids[peer].len());
                push_action(&mut records, &state, "HandleSyncRequest", peer);

                if duplicate_replies.contains(&(other, request_id))
                    && duplicated.insert((other, request_id))
                {
                    let reply = state
                        .network
                        .iter()
                        .find(|message| {
                            message.kind == "SyncReply"
                                && message.to == ModelState::peer_name(other)
                                && message.token == token
                        })
                        .expect("reply selected for duplication exists")
                        .clone();
                    state.network.push(reply);
                    push_action(&mut records, &state, "DuplicateMessage", other);
                }
            },
            HandshakeTraceAction::HandleReply {
                request_id,
                disposition,
            } => {
                let token = *raw_ids[peer]
                    .get(&request_id)
                    .expect("handled reply names an emitted local request");
                let index = state
                    .network
                    .iter()
                    .position(|message| {
                        message.kind == "SyncReply"
                            && message.to == ModelState::peer_name(peer)
                            && message.token == token
                    })
                    .expect("handled reply is present in the modeled network");
                let reply = state.network.remove(index);
                let already_accepted = state.accepted_tokens[peer].contains(&token);
                let phase_before = state.phase[peer];
                if phase_before == "Syncing" && !already_accepted {
                    let remote_config: ModelConfig = ordered
                        .event
                        .remote_config
                        .expect("accepted reply records its remote config")
                        .into();
                    assert_eq!(remote_config, reply.config);
                    state.learned_config[peer] = Some(remote_config);
                    state.learned_from[peer] = Some(ModelState::peer_name(other));
                    if remote_config != state.local_config[peer] {
                        assert_eq!(disposition, HandshakeReplyDisposition::Incompatible);
                    } else {
                        assert_eq!(disposition, HandshakeReplyDisposition::Accepted);
                        state.accepted_tokens[peer].push(token);
                        state.accepted_tokens[peer].sort_unstable();
                    }
                    assert!(outstanding_ids[peer].remove(&request_id));
                } else {
                    let expected_disposition = if phase_before == "Failed" {
                        HandshakeReplyDisposition::AlreadyIncompatible
                    } else if phase_before == "Synced" {
                        HandshakeReplyDisposition::NotSynchronizing
                    } else {
                        HandshakeReplyDisposition::Duplicate
                    };
                    assert_eq!(disposition, expected_disposition);
                    if disposition == HandshakeReplyDisposition::Duplicate {
                        duplicate_reply_step = Some(records.len() - 1);
                    }
                }
                state.apply_runtime_snapshot(ordered, outstanding_ids[peer].len());
                push_action(&mut records, &state, "HandleSyncReply", peer);
            },
            HandshakeTraceAction::ReportTimeout { elapsed_ms } => {
                assert!(elapsed_ms >= 500);
                state.timeout_ticks[peer] = 1;
                push_action(&mut records, &state, "TickTimeout", peer);
                state.apply_runtime_snapshot(ordered, outstanding_ids[peer].len());
                push_action(&mut records, &state, "ReportSyncTimeout", peer);
            },
        }
    }

    NormalizedTrace {
        records,
        duplicate_reply_step,
    }
}

fn write_ndjson(path: &Path, records: &[serde_json::Value]) {
    let mut encoded = String::new();
    for record in records {
        encoded.push_str(&serde_json::to_string(record).expect("trace record serializes"));
        encoded.push('\n');
    }
    std::fs::write(path, encoded).expect("runtime trace is written");
}

#[test]
fn export_runtime_handshake_traces_for_tlc() {
    let matching_events = matching_runtime_events();
    let peer0_actions: Vec<_> = matching_events
        .iter()
        .filter(|ordered| ordered.peer == 0)
        .map(|ordered| ordered.event.action)
        .collect();
    let accepted_ids: Vec<u32> = peer0_actions
        .iter()
        .filter_map(|action| match action {
            HandshakeTraceAction::HandleReply {
                request_id,
                disposition: fortress_rollback::__internal::HandshakeReplyDisposition::Accepted,
            } => Some(*request_id),
            _ => None,
        })
        .collect();
    let duplicate_id = peer0_actions.iter().find_map(|action| match action {
        HandshakeTraceAction::HandleReply {
            request_id,
            disposition: fortress_rollback::__internal::HandshakeReplyDisposition::Duplicate,
        } => Some(*request_id),
        _ => None,
    });
    let sent_ids: Vec<u32> = peer0_actions
        .iter()
        .filter_map(|action| match action {
            HandshakeTraceAction::SendRequest { request_id, .. } => Some(*request_id),
            _ => None,
        })
        .collect();

    assert!(sent_ids.len() >= 3, "matching trace must emit fresh A/B/C");
    assert_eq!(duplicate_id, sent_ids.first().copied());
    assert_eq!(accepted_ids.first(), sent_ids.first());
    assert_eq!(accepted_ids.get(1), sent_ids.get(2));

    let matching = normalize_runtime_trace(
        "runtime-matching",
        "Runtime SimNet matching handshake with duplicate A and accepted C before delayed B.",
        &matching_events,
    );
    let duplicate_step = matching
        .duplicate_reply_step
        .expect("runtime matching trace contains a duplicate reply");
    let mismatch = normalize_runtime_trace(
        "runtime-mismatch",
        "Runtime SimNet handshake between two- and three-player configurations.",
        &mismatch_runtime_events(),
    );
    let timeout = normalize_runtime_trace(
        "runtime-timeout",
        "Runtime SimNet one-shot timeout followed by an enabled retry.",
        &timeout_runtime_events(),
    );

    let matching_final = matching
        .records
        .last()
        .and_then(|row| row.get("updates"))
        .expect("matching trace has a final action row");
    assert_eq!(
        matching_final["phase"],
        serde_json::json!({"p1": "Synced", "p2": "Synced"})
    );
    let mismatch_final = mismatch
        .records
        .last()
        .and_then(|row| row.get("updates"))
        .expect("mismatch trace has a final action row");
    assert_eq!(
        mismatch_final["phase"],
        serde_json::json!({"p1": "Failed", "p2": "Failed"})
    );
    assert!(timeout.records.iter().any(|row| {
        row.get("action") == Some(&serde_json::Value::String("ReportSyncTimeout".to_owned()))
    }));

    let Ok(output_dir) = std::env::var("FORTRESS_RUNTIME_TRACE_DIR") else {
        return;
    };
    let output_dir = Path::new(&output_dir);
    assert!(output_dir.is_dir(), "runtime trace output directory exists");
    write_ndjson(
        &output_dir.join("runtime-matching.ndjson"),
        &matching.records,
    );
    write_ndjson(
        &output_dir.join("runtime-mismatch.ndjson"),
        &mismatch.records,
    );
    write_ndjson(&output_dir.join("runtime-timeout.ndjson"), &timeout.records);

    let duplicate_row = matching
        .records
        .get(duplicate_step + 1)
        .expect("duplicate step maps to an NDJSON row");
    let from = duplicate_row["updates"]["syncRemaining"]["p1"]
        .as_u64()
        .expect("duplicate row has p1 syncRemaining");
    assert_eq!(from, 1, "duplicate A must leave one p1 roundtrip pending");
    let reject = vec![serde_json::json!({
        "schema": 1,
        "trace": "runtime-duplicate-reply-decrement",
        "expect": "reject",
        "description": "Runtime-derived duplicate reply cannot decrement syncRemaining again.",
        "derived_from": "runtime-matching.ndjson",
        "mutation": {
            "step": duplicate_step,
            "variable": "syncRemaining",
            "peer": "p1",
            "from": from,
            "to": 0,
        },
    })];
    write_ndjson(
        &output_dir.join("runtime-duplicate-reply-decrement.ndjson"),
        &reject,
    );
}
