<p align="center">
  <img src="docs/assets/logo-small.svg" alt="Fortress Rollback" width="64">
</p>

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Note:** For historical changes from the original GGRS project (versions 0.2.0–0.11.0),
> see [docs/ggrs-changelog-archive.md](docs/ggrs-changelog-archive.md).

## [Unreleased]

### Changed

- **Breaking:** `ChaosSocket::with_clock()` callbacks now return `web_time::Instant` instead of `std::time::Instant`, allowing the default clock to run on browser `wasm32-unknown-unknown` without panicking. Browser callers with an injected clock must return `web_time::Instant`; native callers require no change because `web_time` re-exports `std::time::Instant` on non-browser targets.

## [0.9.0] - 2026-06-22

### Added

- Spectator failover via `SessionBuilder::start_spectator_session_multi(&[T::Address], socket)`: a spectator can connect to multiple redundant game peers ("hosts") at once. Unresolved frames use the highest-priority currently connected host by builder order as the canonical source; lower-priority host snapshots stay provisional while a higher-priority host is connected, and if the canonical host disconnects before a frame resolves the next surviving host is promoted only for unresolved frames. Connection status is taken from the selected host's whole-frame snapshot rather than merged player-by-player across hosts, then folded into the spectator's persistent per-player connect-status with a reactivation-safe merge rather than raw-overwritten: a dropped slot's freeze frame converges to the global minimum and is never raised by a later canonical host (redundant hosts failing over under asymmetric loss with two or more hosts); this global-min convergence holds regardless of host arrival order or canonical-host selection, folding a lower-`F` host's staged freeze frame down both at commit time (when that host is already staged for the committing frame) and on late arrival (when its lower-`F` stream lands after the frame was committed), so a host that reports an already-dropped slot disconnected at a higher last frame can no longer push the spectator's input-status path back to `Confirmed` for frames the game mesh already froze — the spectator analog of the cross-peer connect-status convergence that keeps `DisconnectBehavior::ContinueWithout` survivors byte-identical — while a genuine reactivation (a `disconnected → connected` transition, as a hot-join host gossips when a dropped slot rejoins) is still followed so the spectator tracks the live game. A dropped slot's frozen input **value** now converges the same way: the spectator surfaces the dropped peer's input at the agreed freeze frame `F` (= the converged `last_frame`) rather than the value the host happened to forward for the frame being advanced, mirroring the host's own frozen-slot semantics. Without this, when a host that detected a drop "high" forwarded a dropped slot's pre-convergence value to its spectator before lower third-peer gossip lowered `F`, the spectator's append-only stream (the host re-sends no already-forwarded frame) stranded it on the stale value and silently diverged from the game mesh even though its status correctly converged; surfacing the value at `F` self-corrects as the connect-status merge lowers `F`, with no host re-send path. The reactivation follow is gated on per-host disconnect-witness provenance: the spectator tracks, per host and slot, the highest freeze frame at which that host's own forwarded connect-status stream has reported the slot disconnected, a canonical host's connected report re-opens a latched drop only when that host has itself witnessed the latched drop (witnessed freeze ≥ latched freeze), and a followed re-open consumes every host's witness for that slot, so a stale lagging host that never observed the drop — or witnessed only an earlier, already-followed drop→rejoin cycle, including at a pre-convergence-high view that numerically covers a later drop's converged freeze — can no longer resurrect a permanently-dropped slot's label when it becomes the canonical source after failover, while genuine hot-join reactivations are still followed (a re-opening host transitions its own gossiped view through the drop first, and gossip rides every packet, so a live witness re-establishes itself after consumption); and a drop adopted at commit time re-arms the committing host's witness at the adopted freeze, so when a stale staged drop snapshot commits after a follow already consumed the witness table (re-freezing the just-reactivated slot with a label the spectator itself authored), that same host's later connected gossip can still re-open the slot instead of leaving it frozen permanently. Known residual: provenance is spectator-local arrival-time observation, so a reactivation gossiped only by a host whose drop-era packets never reached this spectator (or reached it only before the previous follow consumed the witness table, with none after) fails closed at the frozen label rather than resurrecting. Drop cycles are ordered by a per-slot connection-status **epoch** — a `u16` generation each peer bumps on every `connected ↔ disconnected` transition of its own `local_connect_status` (a drop or a reactivation), carried on the connect-status gossip in every input packet — so a report from an earlier or reordered cycle can no longer be mistaken for a fresh one: the spectator tracks, per host and slot, both the high-water generation it has seen and the generation of each witnessed drop, and (1) a reordered earlier-cycle drop report whose generation is below the high-water can no longer re-arm consumed provenance, while (2) a reordered pre-drop connected snapshot whose generation predates the witnessed drop can no longer be followed — closing the cross-cycle stale-re-arm and within-cycle transient resurrects that previously needed this dedicated host→spectator wire signal. The epoch is inert for the player mesh (the confirmed/freeze folds and disconnect convergence read only `disconnected`/`last_frame`); only the spectator consumes it. In the legacy uniform-epoch case (peers that predate the field, all reports at generation `0`) the ordering is inert and the gate reduces exactly to the prior freeze-only provenance behavior. The frozen-value convergence also degrades gracefully if `F` has already aged out of the spectator's input ring buffer (it falls back to the forwarded value, the same `F`-evicted residual the host tolerates). Redundant hosts that disagree on the same player/frame now fail closed: the spectator records a frame-sync violation, emits `FortressEvent::SpectatorDivergence`, and future `advance_frame` calls return `FortressError::SpectatorDivergence` without rewriting frames that already advanced. A clean all-host disconnect with no divergence or malformed canonical data releases the stream-delay boundary so already-buffered frames can drain before `PredictionThreshold`. `SpectatorSession::num_hosts()` reports the current number of connected hosts. The existing single-host `start_spectator_session` delegates to the same construction path and now also rejects invalid `SpectatorConfig` values. If duplicate host addresses are supplied, inbound packets are routed to the first matching host endpoint.
- Enhanced spectator: `SpectatorConfig::stream_delay` holds playback back from the live edge by a configurable number of frames (anti-stream-sniping); the spectator never advances past `last_received_frame - stream_delay`, clamps the delayed boundary at `Frame::NULL` until enough host inputs arrive, and returns `PredictionThreshold` cleanly at the boundary. `SpectatorConfig::enable_rewind` makes the spectator emit a `SaveGameState` each frame and unlocks `SpectatorSession::seek_to_frame(Frame)`, which jumps to any previously simulated frame still in the buffer via a single `LoadGameState` (no re-simulation); seeking on a non-rewind spectator returns `InvalidRequestKind::NotSupported`, a negative target returns `InvalidFrameStructured`/`MustBeNonNegative`, `target + 1` overflow returns `FrameArithmeticOverflow`, and a frame outside the buffered window returns `InvalidFrameStructured`/`MissingState`. New accessors `SpectatorSession::is_rewind_enabled()` and `SpectatorSession::stream_delay()` expose the configured settings. `SpectatorConfig::validate()` rejects only semantic invalid configs (`buffer_size == 0` and `stream_delay >= buffer_size`; `catchup_speed == 0` remains valid), while spectator startup and catch-up request reservation now use fallible allocation so very large user-configured buffers return `None` instead of risking an allocator abort. The spectator advance loop now also returns partial catchup progress instead of discarding already-gathered requests on a mid-batch unavailability; the degenerate `catchup_speed == 0` case (no frame is even attempted while behind) still returns `Ok(<empty>)` rather than `PredictionThreshold`. `seek_to_frame`'s seekable upper bound is `current_frame - 1`: seeking to the exact current frame returns `MissingState` because the post-state for that frame is not saved until the next advance.
- Hardened network input decompression and message decoding against denial of service from hostile length prefixes: protocol receive now derives the accepted RLE decoded length from the user-configured `ProtocolConfig::pending_output_limit` and the reference input size, with `fortress_rollback::rle::DEFAULT_MAX_DECODED_LEN` as a hard upper ceiling; standalone `rle::decode` uses the same public default. Oversized or overflowing decoded lengths return structured errors instead of attempting an unbounded allocation that the default allocator would turn into a process abort. Length accumulation and cursor advancement also use checked arithmetic so crafted packets can no longer overflow `usize`; each run length is range-checked against the limit while still held as `u64` — before any narrowing to `usize` — so the bound cannot be defeated by a 32-bit (`usize == u32`) truncation. Delta decoding also caps the number of decoded frame buffers before allocating, so a tiny reference input cannot fan a bounded byte stream into millions of `Vec` headers; `ProtocolConfig::MAX_PENDING_OUTPUT_LIMIT` keeps valid send batches aligned with that receive cap, and valid senders now split pending input batches to the same byte/frame budget receivers enforce. Per-player `Config::Input` reconstruction now uses bounded bincode decoding when checking consumed byte counts, so a malicious length prefix inside a peer-controlled input slice returns a decode error instead of using generic unbounded bincode allocation. Built-in UDP, Tokio UDP, and chaos sockets cap each receive poll by raw receive attempts and decoded messages, and grow receive batches fallibly. `network::codec::decode_message()` exposes the same bounded `Message` decoder used by the built-in sockets for custom transports and fuzzing, avoiding generic bincode `Vec` decoding on untrusted peer bytes.
- Network input serialization is now validated as a nonzero, fixed-width byte stream for protocol use. Endpoint creation rejects `Config::Input` types whose default value serializes to zero bytes, rejects local or remote aggregate input frames larger than the receive decode cap, and the sender refuses to queue or delta-encode any per-player input value whose serialized length differs from the default/reference width.
- `ReplayDecodeConfig` and `Replay::from_bytes_with_config()` for callers that want to apply their own encoded replay byte limit or skip post-decode validation. `Replay::from_bytes()` now uses the same checked decoder with the default no-byte-limit config and validates decoded metadata before returning. Every length-prefixed allocation in the decoder (the frame vector, each per-frame input vector, and the checksum vector) is now bounded against the remaining byte slice before any memory is reserved, so a malformed replay cannot drive an out-of-proportion allocation regardless of the configured byte limit.
- `network::compression::decode_with_max_len()` for callers and fuzz targets that need an explicit decoded RLE length policy instead of the default `decode()` policy.
- Configurable socket buffer sizes: `UdpNonBlockingSocket::bind_to_port_with_buffer_sizes` / `from_socket_with_buffer_sizes` and (with the `tokio` feature) `TokioUdpSocket::with_buffer_sizes` / `bind_to_port_with_buffer_sizes` let applications size the reused receive and send buffers for larger serialized inputs. All of these reject a zero buffer size with `io::ErrorKind::InvalidInput` (a zero-length socket buffer can never send or receive) and reserve fallibly, so an over-large size returns an error instead of aborting the process. The infallible `TokioUdpSocket::new` degrades to a smaller buffer under allocator pressure rather than producing a permanently unusable socket, and its receive loop never spins on an empty buffer.
- `SessionBuilder::start_p2p_session` now surfaces the underlying `FortressError` from endpoint creation — the IO/socket, protocol, or configuration cause (including `InvalidRequestKind::AllocationFailed`) — instead of forcing every cause to a single opaque `SerializationErrorKind::EndpointCreationFailed` / `SpectatorEndpointCreationFailed`, so callers can distinguish why synchronization setup failed. `EndpointCreationFailed` is still returned for the specific input-serialization failure that produces it; the previously catch-all `SpectatorEndpointCreationFailed` mapping is removed but the variant remains for backward compatibility.
- **`telemetry::push_violation_observer` / `telemetry::ScopedObserverGuard` /
  `telemetry::report_to_current_observer` — a thread-local *scoped* violation-observer primitive.**
  `push_violation_observer(observer)` installs `observer` as the current thread's violation observer as
  long as the returned guard lives; while installed, the `report_violation!` macro routes to it (installs
  nest; the innermost wins; the guard removes its own observer on drop, restoring the previous observer
  for normal LIFO scopes). This is the mechanism that
  makes `with_violation_observer` work for `P2PSession`/`SyncTestSession`/`SpectatorSession` (see *Fixed*), and is also usable
  directly to scope violation routing around arbitrary code. `report_to_current_observer` is the macro's
  dispatch target (it falls back to the `tracing` observer when none is installed).
- **`P2PSession::peer_checksum_mismatch_count(handle)` and an advisory per-peer checksum trust-downgrade
  signal (Byzantine-peer hardening).** With desync detection enabled, the session now tracks, per remote
  peer, how many confirmed-frame checksums failed to match the local history, exposes that count via the
  new method, and logs **one** advisory `WARNING` once a peer's count crosses an internal threshold. This
  is deliberately advisory: the library **never auto-ejects** a peer on checksum mismatch (with two
  endpoints it cannot tell which side is wrong, so dropping a peer risks removing the honest one) — apps
  apply their own policy from the raw count. The `SyncHealth::DesyncDetected` docs are clarified
  accordingly: handle it gracefully (it is a recoverable event), do not hard-panic; a single mismatch is a
  genuine divergence in the trusted-peer model but may be a one-off bad checksum from a malicious/buggy
  peer in an untrusted deployment, where *persistence* is what distinguishes the two.
- **Hot Join (reserved-slot model), behind the new opt-in `hot-join` feature flag.** A peer can join a
  running session by filling a *reserved* (or previously gracefully-dropped) player slot: it synchronizes
  with the host, receives a full game-state snapshot, loads it, and resumes normal rollback play — all
  while the total player count (and therefore the network input wire-width) stays fixed, so existing
  peers' delta-compressed input streams are never disturbed.
  - `SessionBuilder::with_hot_join(bool)`, `SessionBuilder::add_reserved_player(addr, handle)` (host side),
    and `SessionBuilder::start_hot_join_session(socket, host_addr)` (joiner side).
  - `SessionBuilder::with_hot_join_serve_timeout_polls(polls)` (host-side maximum polls a serve stays open
    before aborting; rejects values below `2`), `SessionBuilder::with_hot_join_max_snapshot_wire_bytes(bytes)`
    (host-side cap for the complete encoded snapshot message; defaults to 4 KiB for the built-in UDP
    receive buffer), and `SessionBuilder::with_hot_join_ack_resends(resends)` (joiner-side ack-resend
    budget) tune the handshake's loss/latency envelope; all default to sensible values.
  - `SessionState::HotJoining`; `FortressEvent::JoinRequested { handle, addr }` and
    `FortressEvent::PeerJoined { handle, addr }`; `InvalidRequestKind::PlayerCountMismatch { expected, actual }`.
  - Under `hot-join`, `Config::State` additionally requires `Serialize + DeserializeOwned` (for snapshot
    transfer). This bound only applies when the feature is enabled, so it is **not** a breaking change for
    existing builds. Snapshot deserialization is **both allocation-bounded and recursion-depth-bounded**:
    a hostile length prefix yields an error (never an OOM), and a deeply-nested recursive snapshot yields a
    recoverable decode error (never a stack-overflow abort), so a recursive `Config::State` is safe up to a
    generous nesting limit. (`Config::Input` is bound `Copy`, hence provably non-recursive, so the input
    decode path keeps its direct fast path.)
  - The join handshake is ack-gated and loss/latency tolerant within a bounded envelope: the host
    re-serves the snapshot until acknowledged and only reactivates the slot on the ack, an abandoned join
    can never stall or fail-close the host (it resumes solo with the slot still reserved), and a joiner
    that loses its snapshot or ack fails cleanly (retryable) rather than desyncing. A reserved-slot
    endpoint whose synchronized-but-never-joined peer dies is re-armed on its disconnect timeout, so a
    fresh joiner session from the same address can always be served (a dead joiner cannot permanently
    poison its slot's endpoint). Each such re-arm advances the endpoint's packet-filter `magic` as a
    monotonic per-endpoint era counter, so a still-live peer from *any* recent era can never answer (and
    wedge) the rebuilt handshake — collision is impossible across a 65535-rejoin window, not merely with
    the immediately-previous era. Requires `max_prediction >= 1` (lockstep hot-join is rejected at build
    time).
  - **N-peer meshes (3 or more machines) are supported end-to-end**, for both first-time joins of
    build-time reserved slots and re-joins/reconnections of gracefully-dropped slots: the serving host
    (coordinator) pauses and serves a snapshot captured at its last fully-confirmed frame with **bridge
    inputs** (every slot's confirmed input at the snapshot frame), the surviving mesh agrees on the
    reactivation frame (reopen directives and acks, freeze-convergence gated, with bounded
    abort-and-retry on loss or timeout), and the joiner **buffers** the snapshot, applies it only on the
    mesh-wide join commit, bridges one frame from the snapshot state, and contributes real inputs from
    the activation frame. A buffered join attempt whose commit never arrives is bounded by the
    joiner-side poll budget (`with_hot_join_serve_timeout_polls` on the joiner session): past it the
    joiner tears down terminally — surfacing the conventional coordinator `Disconnected` event — and the
    app retries with a fresh session, never a wedge. Build-time requirements for an N-peer **serving
    host**, mirroring the runtime serve gates as `InvalidRequestKind::NotSupported`:
    `SaveMode::EveryFrame`, zero input delay, and at least one local player; an N-peer **joiner**
    likewise requires `SaveMode::EveryFrame`. Machine count is measured per network address, so
    2-machine couch co-op (multiple handles sharing one remote address) is unaffected by these
    requirements. Spec-violation noise from the legitimate pre-activation windows (the coordinator's
    sub-activation pending stream replayed into a fresh joiner; a reactivated slot's empty input ring
    while its first input is still in flight; lifecycle-close stragglers of the joiner's own concluded
    attempt) is reported at trace level, with full severity kept everywhere else. Likewise, freezing a
    slot that has no agreed freeze frame yet — a build-time reserved slot frozen from frame 0, or any
    drop before a first confirmed input (`Frame::NULL`) — is silent rather than emitting a spurious
    `ViolationSeverity::Warning` on every such freeze, matching the frozen-value-convergence no-op; a
    *non-NULL* freeze frame that is genuinely missing from the input ring (evicted, or never received)
    still warns. A hot-joined session's
    confirmed-input stream toward its own spectator endpoints starts at the loaded snapshot frame (the
    earliest frame a mid-game joiner can serve).
  - A slot that is *cleanly gracefully dropped* (via `P2PSession::remove_player`, or automatically on the
    disconnect timeout under `DisconnectBehavior::ContinueWithout`) on a hot-join-serving host is returned
    to the reserved/frozen state and can be re-filled by a returning peer connecting from the same address,
    exactly like a build-time reserved slot — this is the "previously gracefully-dropped" path above.
    Legacy `disconnect_player` (`Halt`-style) drops, and drops on a host that does not serve hot-joins, are
    not made re-joinable.
- Always-on session metrics: `SessionMetrics` (a cheap, `Copy`, `serde::Serialize` snapshot of cumulative
  counters), exposed via `P2PSession::metrics()` and `SpectatorSession::metrics()`. Counters are plain
  integers updated inline on the paths they measure — no timers, no allocation, no `Instant` — so reads are
  deterministic and WASM-safe. The first surface is **event-queue-overflow accounting**:
  `SessionMetrics::events_discarded_total` counts events dropped because the application drained the bounded
  event queue slower than events arrived, and `events_discarded_by_kind` (an `EventKindCounts`) breaks that
  total down per category so a lost safety-critical notification (a `Disconnected` or `DesyncDetected`) is
  visible rather than silent. Supporting types: `EventKind` (a payload-free mirror of `FortressEvent`'s
  variants, with `as_str()`, `ALL`, and `COUNT`) plus `FortressEvent::kind()` to obtain one, and — under the
  `json` feature — `SessionMetrics::to_json()` / `to_json_pretty()` (the per-kind breakdown serializes as a
  self-describing snake_case-keyed map). `SessionMetrics` is `#[non_exhaustive]`, so later releases can add
  counters without a breaking change. `SessionMetrics` now also carries **rollback and pacing counters** for
  `P2PSession`: `frames_advanced` (total simulation steps), `visual_frames` (forward/rendered advances),
  `resimulated_frames` (frames replayed during rollback — `frames_advanced == visual_frames +
  resimulated_frames` by construction), `rollback_count`, `rollback_depth_histogram` (a new
  `RollbackDepthHistogram` bucketing rollbacks by re-simulated depth `1..=16` then `17_plus`, serialized as a
  self-describing depth-keyed map), `max_rollback_depth`, `prediction_miss_count`, `stall_count` (advances
  throttled by a full prediction window — previously unobservable), `wait_recommendations`,
  `confirmation_lag_current` / `_max` / `_sum` (per-advance samples of how far ahead of the confirmed frame
  the simulation runs), `checksums_compared` / `checksums_matched` / `checksums_mismatched` (desync-detection
  comparisons), and the `event_queue_high_water` / `checksum_history_high_water` container high-water marks.
  All counters are always-on, allocation-free, and updated inline on the paths they measure. A companion
  **per-peer** snapshot, `PeerMetrics` (read via `P2PSession::peer_metrics(handle)`), carries wire-exact
  traffic counters for one remote peer or spectator: cumulative `bytes_sent` / `bytes_received` and
  `packets_sent` / `packets_received`, a per-`MessageKind` breakdown of each direction
  (`messages_sent_by_kind` / `messages_received_by_kind`, both `MessageKindCounts` serializing as a
  self-describing label map, with `total()` equal to the matching packet counter), input
  `input_bytes_pre_compression` / `input_bytes_post_compression` totals, and the instantaneous
  `pending_output_len`, `pending_checksums_len`, `ping_ms`, and `remote_frame_advantage` gauges. `MessageKind`
  is a payload-free mirror of the protocol's wire messages (`as_str()`, `ALL`, `COUNT`); `PeerMetrics` is
  `#[non_exhaustive]` and offers `to_json()` / `to_json_pretty()` under the `json` feature. Byte counts are
  payload-only (they exclude the per-packet UDP/IP header the `NetworkStats::kbps_sent` estimate folds in).
  Spectators expose the same per-host wire metrics via `SpectatorSession::peer_metrics(host_index)`, which
  returns `Option<PeerMetrics>` (`None` for an out-of-range index) — hosts are addressed by dense index in
  `0..num_hosts()` in builder-priority order, since a spectator has no player handles for its upstream hosts.
- With the `hot-join` feature, `P2PSession::hot_join_metrics() -> Option<HotJoinMetrics>` reports a joiner's
  hot-join handshake latency: `completed` (whether the joiner applied the host snapshot and reached
  `Running`), `polls_to_running` (`poll_remote_clients` iterations spent `HotJoining`), and
  `millis_to_running` (elapsed time on the injectable protocol clock, so it is deterministic under the
  simulation harness). Returns `None` for any session that did not hot-join (a host, or a peer that
  synchronized normally). `HotJoinMetrics` is `#[non_exhaustive]`, `Copy`, and offers `to_json()` /
  `to_json_pretty()` under the `json` feature.

### Changed

- **Breaking:** `SpectatorConfig` now has public `stream_delay` and `enable_rewind` fields. Existing exhaustive struct literals must add those fields or use `..SpectatorConfig::default()` / `..Default::default()`.
- **Breaking:** `FortressEvent::SpectatorDivergence { frame, player, primary_addr, conflicting_addr }` — new variant added; emitted when connected redundant spectator hosts provide conflicting input for the same player/frame. Since `FortressEvent` is not `#[non_exhaustive]`, exhaustive matches must now handle this variant.
- **Breaking:** `FortressError::SpectatorDivergence { frame, player }` — new variant added; returned by failover spectators after redundant connected hosts disagree on the same player/frame. Since `FortressError` is not `#[non_exhaustive]`, exhaustive matches must now handle this variant.
- **Breaking:** `RleDecodeReason::MalformedVarint { offset }`, `RleDecodeReason::DecodedLengthExceedsMaximum { decoded_len, max }`, and `RleDecodeReason::AllocationFailed { requested_len }` — new variants added; returned when malformed input has an invalid RLE varint length prefix, declares a decoded length above the configured/default limit, or when decoded output reservation fails. Since `RleDecodeReason` is not `#[non_exhaustive]`, exhaustive matches must now handle these variants.
- **Breaking:** `DeltaDecodeReason::DecodedFrameCountExceedsMaximum { frame_count, max }` and `DeltaDecodeReason::AllocationFailed { context, requested_elements }` — new variants added; returned when delta decoding would produce too many per-frame buffers or when delta-decoded output reservation fails. Since `DeltaDecodeReason` is not `#[non_exhaustive]`, exhaustive matches must now handle these variants.
- **Breaking:** `ProtocolConfig::pending_output_limit` is now validated against `ProtocolConfig::MAX_PENDING_OUTPUT_LIMIT`; larger values return `InvalidRequestKind::ConfigValueOutOfRange` instead of creating sessions that can emit undecodable input batches.
- **Breaking:** `InternalErrorKind::DeltaEncodeEmptyReference`, `InternalErrorKind::DeltaEncodeInputLengthMismatch { input_len, reference_len }`, and `InternalErrorKind::InputEncodeLengthMismatch { player, input_len, expected_len }` — new variants added; reported when internal delta/input encoding is asked to emit a batch the receiver cannot decode. Since `InternalErrorKind` is not `#[non_exhaustive]`, exhaustive matches must now handle these variants.
- **Breaking:** `SerializationErrorKind::InputSerializedSizeZero` and `SerializationErrorKind::InputSerializedFrameTooLarge { frame_len, max }` — new variants added; returned by network endpoint creation when `Config::Input::default()` serializes to zero bytes or when a local/remote aggregate serialized input frame would exceed the receive decode cap. Since `SerializationErrorKind` is not `#[non_exhaustive]`, exhaustive matches must now handle these variants.
- **Breaking:** `InvalidRequestKind::AllocationFailed { context, requested_elements }` — new variant added; returned when user-configured session sizes cannot be reserved fallibly. Since `InvalidRequestKind` is not `#[non_exhaustive]`, exhaustive matches must now handle this variant.
- **Breaking:** `Replay::from_bytes()` now requires `I: Copy`, matching the `Config::Input` contract used by replay sessions, and rejects malformed/trailing replay bytes or validation failures instead of returning unchecked decoded data.

### Fixed

- **Pre-existing:** `NetworkStats::kbps_sent` is now accurate on both axes it was previously wrong on.
  (1) *Payload size:* it accounted `std::mem::size_of_val(&Message)` — the constant in-memory size of
  the `Message` enum, identical for every variant (dominated by the largest one) and independent of
  the actual payload, since `Vec` contents live on the heap, so a bare `KeepAlive` was charged the
  same as a fully-loaded `Input`. Sending is now metered with a new alloc-free arithmetic
  `Message::encoded_len()` that is byte-exact against the codec (a property test asserts
  `encoded_len() == codec::encode(&msg).len()` for arbitrary messages of every variant). (2) *Unit:*
  the field is documented and named as kilobits per second, but the rate was computed as
  `bytes / seconds / 1024` — kibibytes per second, off by the 8x byte-vs-bit factor. It is now
  computed as bits (x8) per second / 1000, matching the documented unit. The `+ UDP_HEADER_SIZE`
  per-packet header estimate is unchanged. Net effect: reported values change (they were previously
  fiction); code that hard-coded a calibration against the old numbers should re-baseline.
- **Pre-existing:** Closed a permanent whole-mesh confirmation deadlock at 3 or more players (the
  no-drop sibling of the 0.9.0 gossip-mute fix), found by the new deterministic whole-mesh
  simulation fleet on its first four-player run. Connect-status gossip rides only `Input` messages;
  if a peer sent its entire initial prediction window's inputs before hearing a third peer's first
  input (transient startup loss, jitter, or reordering is enough), its last gossip left that slot's
  view at `Frame::NULL` in every receiver's cache. Once every peer exhausted its prediction window
  (`current - confirmed >= max_prediction`) with fully-acked send queues, no peer ever sent another
  `Input` — `KeepAlive`, `QualityReport`, and `InputAck` carry no connect status — so the stale
  caches never refreshed and the whole mesh deadlocked **permanently** at `confirmed_frame() ==
  Frame::NULL` on a perfectly healed network, with every session `Running` and no error or event
  (an application-visible infinite freeze). The connect-status nudge introduced in 0.9.0 closed
  exactly this gossip-mute mechanism but only while a locally-detected drop awaited mesh agreement;
  it now also arms when **both** legs of the deadlock signature hold: the prediction window is
  exhausted AND the mesh-gossip fold pins confirmation strictly below what locally-received inputs
  alone would allow (gossip, not receipts, is the binding constraint). While armed, each input-idle
  endpoint re-sends one status-bearing duplicate input per keepalive interval (the existing nudge
  wire shape — receivers already treat it as a stale retransmission; no wire-format change), so
  every peer's contribution to the others' folds catches up to its real receipts and confirmation
  releases. A healthy mesh's normal one-gossip-delivery pacing lag never exhausts the prediction
  window, receipt-bound stalls (a peer's inputs genuinely missing) still belong exclusively to
  pending-output retransmission, and reserved hot-join endpoints are never nudged (a duplicate
  `Input` injected into the join handshake interferes with the joiner's deferred input processing),
  so actively-advancing sessions' packet streams are unchanged. Two-player sessions were never
  affected (the fold collapses to the local receipt).
- **Pre-existing:** `NetworkStats::ping` (the quality-report round-trip time) is now measured on the
  protocol's monotonic clock — honoring an injected `ProtocolConfig::clock` — instead of the system
  wall clock. Wall-clock adjustments (NTP steps, VM snapshot restores) can no longer corrupt the
  reported RTT, and sessions driven by a virtual clock (deterministic tests and simulations) now
  measure virtual network latency exactly and reproducibly instead of leaking wall-clock scheduling
  noise. The previous behavior of silently skipping a quality report or RTT update while the system
  clock was in an abnormal state is gone entirely (a monotonic elapsed reading cannot fail). The wire
  format is unchanged — the peer echoes the timestamp verbatim, so mixed builds across this change
  interoperate — and as a consequence `js-sys` is no longer a dependency on `wasm32` targets (its
  only use was reading the wall clock for these timestamps).
- `SessionBuilder::with_violation_observer` now routes specification violations to the
  configured observer for **every** session type — `P2PSession`, `SyncTestSession`, and `SpectatorSession`.
  Previously `P2PSession` and `SyncTestSession` emitted every specification violation through the global
  `tracing`-backed observer only, so a `ViolationObserver` (e.g. `CollectingObserver`) attached via
  `with_violation_observer` never received them — the released API promise was silently false for the two
  most common session types. `SpectatorSession` already routed the violations raised on its direct paths,
  but a residual set of `report_violation!` sites (the `frames_behind_host`, `inputs_at_frame`, and
  host-input validation guards) bypassed the per-session observer too. Each session now installs its
  configured observer as a thread-local scope for the duration of every public, state-driving entry point
  and during construction (`P2PSession`: `advance_frame`, `poll_remote_clients`, `add_local_input`,
  `disconnect_player`, `remove_player`, `set_input_delay`; `SpectatorSession`: `advance_frame`,
  `poll_remote_clients`, `seek_to_frame`, `frames_behind_host`), so all `report_violation!` calls beneath —
  including those raised by the low-level input-queue, sync-layer, and protocol components — route to the
  per-session observer. When no observer is configured, violations fall back to the `tracing` observer
  exactly as before, so there is no behavior change for sessions that do not opt in.
- A prediction episode in the input queue now always begins at the queue's **first
  missing frame** rather than at the frame the simulation happened to request. Previously, a rollback re-simulation
  whose first input request landed **above** a remote queue's missing window — ordinary cross-endpoint
  jitter or packet reordering in sessions of 3 or more players, reachable at 0% packet loss with no
  disconnects — re-entered prediction at the requested frame, and the missing window's inputs were then
  accepted into the queue with their misprediction check **silently skipped**, so no rollback ever
  re-simulated those frames. Two user-observable symptoms are fixed: (1) the affected peer's applied
  game state could silently and permanently diverge from the rest of the session — no rollback, no
  error, no event — which with `DesyncDetection::Off` was completely invisible; and (2) with
  `DesyncDetection::On`, checksums harvested from the divergent peer's saved states raised
  false-positive `DesyncDetected` events on every peer even though all peers' confirmed input streams
  were byte-identical. The queue's internal frame/prediction mismatch arm now also fails toward a
  corrective rollback instead of silently skipping the comparison. (The prediction-entry semantics are
  observable through the `__internal::InputQueue` testing/fuzzing surface, which is documented as not
  part of the stable public API, so this is a fix rather than a breaking change.)
- Closed (at 3 players) and strictly narrowed (at 4 or more) the graceful peer drop
  *staggered detection* residual documented (as "not yet closed") in 0.8.1's `DisconnectBehavior` entry.
  Under `DisconnectBehavior::ContinueWithout` in sessions of 3 or more players, a survivor that had received
  more of a dropping peer's inputs than another (asymmetric packet loss) could confirm frames past the
  eventual mesh-agreed freeze frame `F` before the lowering knowledge arrived, in two orderings: after
  locally detecting the drop first (the dropped slot left the confirmed-frame minimum immediately), or with
  **no disconnect knowledge anywhere** (the slot's locally-received frame simply ran ahead of the lagging
  survivor's). The damage came in two mechanisms: the race let the simulation run so far that the `F + 1`
  rollback target fell below the prediction-window floor, so the clamped re-simulation left every frame in
  `(F, floor)` permanently embedding the dropped peer's unagreed high-frame inputs (the common case), and at
  extreme stagger (128 or more frames) the input ring physically overwrote the input at `F` so the
  convergence re-roll fail-safed into a stale value. Either way that survivor's confirmed history diverged
  for every frame above `F` — completely silent with `DesyncDetection::Off`, a `DesyncDetected` event
  otherwise. `P2PSession::confirmed_frame()` now implements the GGPO-faithful freeze barrier (upstream
  `PollNPlayers` semantics): a still-connected remote slot contributes the minimum of the locally received
  frame **and every running remote endpoint's gossiped view of that slot** (the same fold the
  disconnect-convergence machinery uses); a **locally disconnected** slot whose drop is not yet mesh-agreed
  contributes the gossiped views **only** (the local detection value is dropped from the fold, exactly as
  GGPO skips `local_connect_status` when disconnected — required for liveness so a survivor capped against
  its own detection value is not pinned by it); and a disconnected slot is excluded from the minimum only
  once its drop is **mesh-agreed** (no running endpoint still reports it connected); hot-join reserved
  endpoints with their freshly reset status caches are skipped so an unfilled or abandoned join cannot pin
  the bound. Because connect-status gossip rides only input messages, survivors that detect a drop while
  capped at their prediction window with fully-acked send queues used to be gossip-mute — mesh agreement
  could then never be reached (a permanent, silent stall in the common clean-drop case, and mutual deadlock
  between the non-minimum survivors at 4 or more players). Input-idle endpoints (no input packet sent for a
  keepalive interval, empty send queue) therefore now emit a periodic **connect-status nudge** while a drop
  awaits mesh agreement: a status-bearing duplicate input packet (re-sent from the already-acked delta
  reference on the keepalive cadence, an existing wire shape that receivers already handle as a stale
  retransmission), so the disconnect gossip always has a carrier and the liveness hold is bounded by the
  disconnect timeout plus the nudge cadence and delivery; while real input traffic flows the nudge stays
  completely silent, so an actively-advancing session's packet stream is unchanged. To keep the
  post-agreement leg of that bound honest, a received input packet now refreshes the pending-input
  retransmission timer only when it stages at least one new frame: progress-free duplicates (nudges
  included) no longer suppress a survivor's resend of its still-unacked inputs — the only remaining carrier
  of its connect-status view once its own nudging stops — while disconnect-timeout tracking still counts
  every packet (under duplicate-heavy loss this means at most one extra resend per retry interval). Observable pacing
  change at 3 or more players: `confirmed_frame()` now reports the mesh-gossip minimum at all times, never
  just the local receipt — even a healthy steady state permanently paces roughly one gossip delivery behind
  the local receipt (GGPO `PollNPlayers` parity), and asymmetric loss widens that gap until gossip catches
  up or the mesh agrees a slot is down — exactly the moments the old, higher value was unsafe to act on —
  and the reported value is no longer guaranteed monotonic call-to-call; while held, `advance_frame` paces
  via the normal
  prediction-window throttle (`Ok` with no `AdvanceFrame` request), never an error, and the hold releases
  via gossip delivery (nudged when idle), the propagated-disconnect path, or a dead endpoint timing out of
  the fold. At 2 players the reported value is byte-identical to the previous behavior in normal operation
  (the peer's self-claim always covers the inputs it sent, and a dropped peer's terminal endpoint leaves the
  fold), with two named conservative transient windows documented in the source (peer-initiated disconnect
  packets, and hot-join reactivation reporting `Frame::NULL` until the joiner's first input). Remaining
  residuals (documented in the source, all requiring 4 or more players, none a regression): the
  "stale-echo" race (a third survivor freezing the dropped slot from a stale cache of another survivor's old
  claim) was arbitrated **not a bug** — a still-running endpoint's stale-low term is folded into the
  confirmed bound and the freeze override identically, so confirmation is pinned at the freeze and never
  overruns it. The **double-failure-relay corner** (an origin survivor dying mid-relay, leaving a window
  where the bound exceeds the later-relayed override) is now **closed** by a sequence-numbered
  **floor-round** (two new wire messages, `FloorRequest`/`FloorReply`). When a relay topology forms — a
  survivor has pruned a remote **and** at least two remotes still run — the survivor issues `FloorRequest`s
  to its folded relays; each relay answers with its current per-slot **pessimistic floor** (the `min` over
  its own freeze/receipt and the committed freezes of the peers it folds disconnected — a departed origin's
  relayed low), and the survivor holds its confirmed bound for a connected relay's slot at the current
  confirmed frame until a reply that postdates its most recent prune has landed from every folded relay,
  then folds the replied floors — never confirming and discarding the dropped slot's real inputs above a
  freeze the relaying survivor will agree to. The reply rides a **dedicated, reorder-immune channel** (a
  per-request sequence number drops a reordered/stale reply; the observer also drops a reply echoing a
  sequence it never issued, and any reply that omits a slot, so an accepted reply always fully and freshly
  defines the cached floors), so it closes not only the warm / in-order
  case but also the cold-cache case (a relay whose floor the observer never gossiped — reachable under
  hot-join reactivation) and the mid-game-drop **reorder** case (a stale-high floor reordered on the wire),
  which a floor gossiped on the input packet could not. The round runs only inside the relay topology, so
  the steady state and 2-/3-peer post-drop pacing are unchanged. **This is a wire-format addition (the
  `FloorRequest`/`FloorReply` message variants), so all peers in a session must run a wire-compatible
  build** (mixed builds across this change cannot exchange these messages; they ride the same unversioned
  framing as the existing protocol messages). Byzantine peers are out of scope. The existing
  defense-in-depth — the prediction-window rollback clamp, the sparse earlier-checkpoint search, and the
  disconnect-rollback checksum invalidation/deferral — is unchanged and now mostly dormant.
- **Pre-existing:** Event-queue overflow no longer discards events **silently**. When the bounded event
  queue exceeds its configured size the session still drops the oldest events (unchanged retention — that
  policy is revisited separately), but each drop is now recorded in `SessionMetrics` (see *Added*) and a
  rate-limited `Warning`/`NetworkProtocol` violation is reported once per overflow episode (re-armed each
  time the application drains events), so a churn burst warns once rather than once per message. Previously the
  oldest event — even a safety-critical `Disconnected` or `DesyncDetected` — was dropped with no violation
  and no counter, so an application draining events slower than they arrive could miss a disconnect or a
  desync with no way to detect the loss. Both `P2PSession` and `SpectatorSession` are covered. The cap is
  now enforced at **every** event-emission path — including events raised from `advance_frame` (wait
  recommendations, desync detection), `remove_player`/`disconnect_player`, and the hot-join lifecycle —
  not only when an inbound message is handled, so the queue can no longer sit above its configured size
  (nor lose events untracked) between polls. Drain events every poll, or raise the event-queue size, to
  keep `events_discarded_total` at zero.

## [0.8.1] - 2026-05-16

### Added

- `P2PSession::set_input_delay()` and `P2PSession::input_delay()` for runtime input-delay adjustment, enabling hybrid delay+rollback in response to changing network conditions. Mid-session **increases** are supported on peers with a single local player: the input queue replicates the most recently added input across the new gap, and the same replicated frames are pushed onto every remote endpoint's pending-output buffer so the remote peer's input sequence remains strictly monotonic. Mid-session **decreases** return an `InputDelayDecreaseUnsupported` error and leave the queue unchanged; mid-session increases on peers with multiple local players return `InputDelayMidSessionMultiLocalUnsupported`. If mid-session increase gap-fill fails an internal queue invariant, the input queue restores its prior state and delay before returning `InputQueueGapFillFailed`. The mid-session gap-fill mirror step now surfaces `InternalErrorStructured(ConnectionStatusIndexOutOfBounds { player_handle })` if the matching `local_connect_status` entry is missing rather than silently skipping the update; if this error is returned through the public API, it indicates an internal-invariant violation and should be treated as a library bug. On a **frozen** input queue (a dropped peer under `ContinueWithout`), `set_input_delay` is unconditionally a silent no-op — including when the requested delay would exceed `max_frame_delay()`, which no longer leaks a `FrameDelayTooLarge` error for an already-gone peer.
- `DisconnectBehavior` enum (`Halt`, `ContinueWithout`) controlling how a `P2PSession` reacts when a remote peer's **automatic** disconnect-timeout fires or when disconnect knowledge is propagated by another peer. `Halt` (default) preserves the legacy GGRS-style halt-on-drop semantics; `ContinueWithout` enables graceful peer drop where remaining peers continue advancing while the dropped peer's input queue is frozen, repeating the dropped peer's input at an **agreed freeze frame** `F` — the global minimum across surviving peers of the dropped slot's last received frame. Under asymmetric packet loss survivors may have received the dropped peer's inputs through different frames; freezing every survivor at the global-min `F` (rather than at each survivor's own last-received value) guarantees they repeat the identical confirmed value, closing the silent desync for the common case. The frozen value converges to `F` on all detection paths (automatic disconnect-timeout, propagated cross-peer disconnect, and explicit `remove_player`): whenever the disconnect machinery lowers the agreed frame toward the global min, the frozen value is re-rolled down to track it. Convergence holds even when the global-min value must transit a relaying peer: a relayed *lowering* of a dropped slot's freeze frame is no longer clobbered by the per-endpoint connect-status merge (which previously took a monotone maximum and silently discarded the lower value), so with three or more survivors under asymmetric loss — where the lowest last-received frame reaches a survivor only second-hand through a peer whose cached view was already higher — every survivor still settles on the same global-min `F` rather than freezing the dropped slot at divergent frames. That relayed disconnect gossip is now also processed off the input-decode path: the per-endpoint connect-status merge runs before the two `on_input` decode-skip branches (a too-far-ahead packet whose intermediate frames are missing, and a stale retransmission whose decode reference has aged out of the receive window), so a peer's lowering gossip converges from **every** received packet rather than only from packets whose delta-compressed inputs happen to decode — narrowing the window of the staggered-detection residual below. The merge is loss/reorder safe by construction (monotone-min for a both-disconnected freeze, never re-raising a converged freeze; first-seen-disconnect adopts the relayed authoritative frame; monotone-max for a connected slot), so applying it from an undecodable or stale packet can never move a survivor's cached view in an unsafe direction. **Known residual (not yet closed):** if survivors detect the drop with enough staggering that one of them confirms *and discards* frames below the eventual global-min `F` before the lowering gossip arrives, that survivor can no longer roll back to `F`; the re-roll then leaves it at its last agreed value (a `ViolationSeverity::Warning` is logged). Default `DesyncDetection::On` surfaces the resulting divergence as a `DesyncDetected` event; with detection `Off` it is logged but otherwise silent. Fully closing this worst case requires a GGPO-style freeze barrier (holding confirmed-frame advance for the dropped slot until `F` is agreed across all survivors) and remains tracked work. As a **liveness** guarantee for the related case where the lowered `F` lands more than `max_prediction` frames behind a survivor that has advanced far ahead, the disconnect-induced rollback target is clamped to the prediction-window floor so `advance_frame` stays live (previously it returned `OutsidePredictionWindow` on every call and the session stalled permanently); the frames below the window remain unrecoverable, exactly as described for the residual above. Under `SaveMode::Sparse` the disconnect-induced rollback no longer re-simulates from a contaminated checkpoint: when a gossip-lowered freeze frame drives the rollback target below the sole tracked `last_saved_frame` (whose state embeds the dropped peer's pre-convergence inputs), the rollback now prefers an earlier sparse checkpoint still buffered at or below the agreed frame `F` — where the dropped slot's value is identical on every survivor — so the re-simulated confirmed history converges across survivors instead of permanently diverging at a fixed offset (only when no such earlier checkpoint remains buffered does the in-window gap stay unrecoverable, consistent with the residual above). This setting governs only automatic/protocol-observed disconnect paths; explicit `P2PSession::disconnect_player` always preserves halt semantics, and explicit `P2PSession::remove_player` always performs a graceful drop, regardless of this setting. Telemetry note: in the `Event::Disconnected` `ContinueWithout` auto-drop path, `freeze_player` failures are reported with `ViolationSeverity::Error`, the endpoint is still marked disconnected, and the session fails closed by returning to `Synchronizing`. The same fail-closed transition now also fires when applying a propagated disconnect (`update_player_disconnects`) or a direct `Event::Disconnected` returns an internal error before disconnect bookkeeping completes — preventing the session from continuing to advance frames after a disconnect observation has been lost. With `DesyncDetection::On`, the freeze re-roll no longer produces a spurious `DesyncDetected`: because re-rolling the dropped slot to `F` retroactively changes the correct checksum at every confirmed frame above `F`, any locally stored (and possibly already sent) checksum for those frames is now invalidated when the disconnect rollback is armed, so a survivor's correct post-convergence checksum is no longer compared against a stale pre-convergence value (the checksum at `F` itself is unchanged and kept); additionally, while a disconnect rollback is armed, no new checksum is sent or stored for a frame that rollback will re-simulate (it is deferred until after re-simulation), so neither a stale local history entry nor a stale gossiped checksum is produced for that frame on the drop-processing advance. Synchronization verification is also tracked per remote peer rather than session-globally: `P2PSession::sync_health(peer)` only reports `InSync` once **that** peer has sent a matching checksum, `is_synchronized()` requires every currently-**connected** remote to be individually verified — a gracefully-dropped or reserved hot-join remote does not block it (so the `confirmed_frame() >= target && is_synchronized()` exit gate still completes after a drop) — and `P2PSession::last_verified_frame()` now returns the highest frame verified with **all** currently-connected remotes (the minimum across them, excluding the same disconnected and reserved hot-join slots), returning `None` while any connected remote is still unverified. `sync_health(peer)` itself stays per-peer truthful and is not connection-filtered. Previously a single verified peer made all peers report `InSync` and `last_verified_frame()` report that peer's frame — benign at two players but a logical error with three or more.
- `SessionBuilder::with_disconnect_behavior()` to opt in to graceful peer drop on a `P2PSession`.
- `P2PSession::remove_player()` for explicit graceful removal of a remote peer. Marks every non-spectator player handle owned by the dropped endpoint as disconnected and freezes each one's input queue (so remaining peers repeat each handle's input at the agreed freeze frame `F` forever, with `InputStatus::Disconnected`; see `DisconnectBehavior::ContinueWithout` above for the global-min `F` agreement that keeps survivors byte-identical under packet loss) — multi-handle endpoints (multiple players sharing a single remote address) are handled in full. Disconnects the network endpoint and emits one `FortressEvent::PeerDropped` per non-spectator handle followed by exactly one address-level `FortressEvent::Disconnected` in the same batch. The call is transactional: if the freeze step cannot succeed for every handle, no state-mutating work is performed. Internal-invariant `freeze_player` failures (pre-validated via `SyncLayer::validate_freeze_player`) are surfaced via `Result::Err` (`InternalErrorStructured` with `IndexOutOfBounds` or `InvalidPlayerHandle`) rather than silently returning `Ok(())` with a `ViolationSeverity::Warning`; reaching this branch indicates a library bug. Always opts in to graceful-drop semantics regardless of the configured `DisconnectBehavior` (which only governs the **automatic** disconnect-timeout path). Distinct from the legacy `disconnect_player()`, which preserves the halt-on-drop semantics.
- `P2PSession::disconnect_behavior()` accessor returning the configured `DisconnectBehavior`.

### Changed

- **Breaking:** `FortressEvent::InputDelayRecommendation { player_handle, current_delay, suggested_delay }` — new variant added; reserved for application-level heuristics or a future automatic emitter (no built-in emitter currently produces it). Since `FortressEvent` is not `#[non_exhaustive]`, exhaustive matches must now handle this variant.
- **Breaking:** `FortressEvent::PeerDropped { handle, addr }` — new variant added; emitted when a remote peer is gracefully removed (auto-removal under `DisconnectBehavior::ContinueWithout` after a timeout, or via explicit `P2PSession::remove_player`). Since `FortressEvent` is not `#[non_exhaustive]`, exhaustive matches must now handle this variant.
- **Breaking:** `InvalidRequestKind::InputDelayDecreaseUnsupported { current, requested }` — new variant added; returned when `set_input_delay` is called with a smaller delay after inputs have been added. Since `InvalidRequestKind` is not `#[non_exhaustive]`, exhaustive matches must now handle this variant.
- **Breaking:** `InvalidRequestKind::InputDelayMidSessionMultiLocalUnsupported { local_players }` — new variant added; returned when attempting to increase input delay mid-session on a peer that hosts more than one local player. Since `InvalidRequestKind` is not `#[non_exhaustive]`, exhaustive matches must now handle this variant.
- **Breaking:** `InvalidRequestKind::InputDelayMidSessionPendingOutputFull { delta, capacity }` — new variant added; returned when a mid-session input-delay increase would enqueue more gap-fill frames than the configured `pending_output_limit` allows. Since `InvalidRequestKind` is not `#[non_exhaustive]`, exhaustive matches must now handle this variant.
- **Breaking:** `InvalidRequestKind::PlayerAlreadyRemoved { handle }` — new variant added; returned by `P2PSession::remove_player` when called twice for the same handle. Since `InvalidRequestKind` is not `#[non_exhaustive]`, exhaustive matches must now handle this variant.
- **Breaking:** `InternalErrorKind::InputQueueGapFillFailed { frame }` — new variant added; reported if mid-session gap-fill replication fails an internal invariant. Since `InternalErrorKind` is not `#[non_exhaustive]`, exhaustive matches must now handle this variant.
- **Breaking:** `PlayerRegistry::handles_by_address` and `PlayerRegistry::handles_by_address_iter` now take `&T::Address` instead of `T::Address`. The same change applies to the `P2PSession::handles_by_address` and `P2PSession::handles_by_address_iter` forwarders. Existing callers passing an owned address must add a leading `&`: `session.handles_by_address(&addr)` and `session.handles_by_address_iter(&addr)`.

> *Follow-up:* a session-level telemetry hook for input-delay changes (e.g.,
> a `TelemetryEvent::InputDelayChanged`) is intentionally deferred to the
> upcoming frame-advantage-heuristic feature, which will be the primary
> producer of input-delay adjustments.

## [0.8.0] - 2026-04-25

### Changed

- **Breaking:** `FortressEvent::ReplayDesync` — new variant added. Since `FortressEvent` is not `#[non_exhaustive]`, exhaustive matches must now handle this variant.
- **Breaking:** `InvalidFrameReason::ReplayExhausted` — new variant added. Since `InvalidFrameReason` is not `#[non_exhaustive]`, exhaustive matches must now handle this variant.
- **Breaking:** `Config::Input` now requires `Eq` in addition to `PartialEq`. Types used as `Config::Input` must derive or implement `Eq`. This ensures reflexive equality, which is a correctness requirement for deterministic rollback — non-reflexive types (e.g., floats with `NaN`) would cause phantom prediction misses and unnecessary rollbacks. All integer and struct-of-integer types already implement `Eq`; add `#[derive(Eq)]` to any custom input types that are missing it.

### Added

- `ReplaySession::new_with_validation()` constructor that enables checksum validation mode, emitting `SaveGameState` requests, comparing checksums against the replay recording, and flushing final-frame validation when `events()` is drained after completion
- `ReplaySession::is_validating()` accessor to check if checksum validation mode is enabled
- `SessionBuilder::start_replay_session_with_validation()` builder method for creating a validation-enabled replay session
- `SessionTelemetry` trait for observing session performance events (rollbacks, prediction misses, frame advances, network stats)
- `CollectingTelemetry` test helper that accumulates `TelemetryEvent` values for assertions
- `TelemetryEvent` enum with `Rollback`, `PredictionMiss`, `NetworkStatsUpdate`, and `FrameAdvance` variants
- `SessionBuilder::with_telemetry()` to attach a telemetry observer to P2P sessions
- `Replay<I>` type for recorded match data with `to_bytes()` / `from_bytes()` serialization using deterministic bincode codec
- `ReplayMetadata` type containing library version, player count, total frame count, and skipped frame count
- `ReplaySession<T>` session type implementing `Session<T>` for deterministic replay playback
- `SessionBuilder::with_recording(bool)` to enable input recording (including game state checksums) during P2P sessions
- `SessionBuilder::start_replay_session(replay)` to create a replay playback session
- `P2PSession::is_recording()` to check if replay recording is active
- `P2PSession::into_replay()` to extract the recorded `Replay` after a session ends (consumes the session)
- `P2PSession::take_replay()` to extract the recorded `Replay` without consuming the session (recording stops after extraction)
- `Replay::validate()` to verify internal consistency of replay data
- Re-exports `Replay`, `ReplayMetadata`, and `ReplaySession` in prelude

## [0.7.0]

### Added

- `ClockFn` type alias (`Arc<dyn Fn() -> Instant + Send + Sync>`) for injectable time sources, enabling deterministic time control in tests and simulations
- `ProtocolConfig::clock` field for overriding the system clock in the network protocol, allowing deterministic simulation testing (DST) and controlled time progression
- `ChaosSocket::with_clock()` builder method for injecting a custom clock into the chaos socket, enabling deterministic latency simulation

### Changed

- **Breaking:** `ProtocolConfig` no longer implements `Copy` due to the addition of the `clock` field (`Option<Arc<dyn Fn>>`). Use `.clone()` instead where `Copy` was previously relied upon.

## [0.6.0]

### Added

- `Session<T: Config>` trait — unified interface for `P2PSession`, `SpectatorSession`, and `SyncTestSession`, enabling generic code that works with any session type
- `RequestVec<T>` — stack-allocated `SmallVec<[FortressRequest<T>; 4]>` for frame advance requests, avoiding heap allocation in the common case
- `EventDrain<'_, T>` — zero-allocation opaque iterator for session events, replacing direct `std::collections::vec_deque::Drain` exposure
- `SyncTestSession::events()` — drain pending events for API consistency with `P2PSession` and `SpectatorSession` (currently always empty; enables future desync-detection events)
- `InvalidRequestKind::NotSupported` variant for operations not supported by a particular session type (e.g., `add_local_input` on a spectator session)

### Changed

- **Breaking:** `P2PSession::advance_frame()` now returns `FortressResult<RequestVec<T>>` instead of `Result<Vec<FortressRequest<T>>, FortressError>`. `RequestVec` implements `Deref<Target = [FortressRequest<T>]>` and `IntoIterator`, so most code (including `handle_requests!`) works unchanged. Use `.to_vec()` if you need a `Vec`.
- **Breaking:** `SpectatorSession::advance_frame()` now returns `FortressResult<RequestVec<T>>` instead of `Result<Vec<FortressRequest<T>>, FortressError>`
- **Breaking:** `SyncTestSession::advance_frame()` now returns `FortressResult<RequestVec<T>>` instead of `Result<Vec<FortressRequest<T>>, FortressError>`
- **Breaking:** `P2PSession::events()` now returns `EventDrain<'_, T>` instead of `std::collections::vec_deque::Drain<'_, FortressEvent<T>>`
- **Breaking:** `SpectatorSession::events()` now returns `EventDrain<'_, T>` instead of `std::collections::vec_deque::Drain<'_, FortressEvent<T>>`
- **Breaking:** Added `InvalidRequestKind::NotSupported` variant for unsupported session operations. Exhaustive matches on `InvalidRequestKind` must now handle this new variant.

## [0.5.0]

### Added

- `HandleVec` type alias — stack-allocated `SmallVec<[PlayerHandle; 8]>` for zero-allocation player handle queries
- Zero-allocation iterator methods for `PlayerRegistry`:
  - `local_player_handles_iter()` — iterate over local players without allocation
  - `remote_player_handles_iter()` — iterate over remote players without allocation
  - `spectator_handles_iter()` — iterate over spectators without allocation
  - `all_player_handles_iter()` — iterate over all handles without allocation
  - `handles_by_address_iter(addr)` — iterate over handles by address without allocation
- Zero-allocation iterator methods for `P2PSession`:
  - `local_player_handles_iter()` — iterate over local players without allocation
  - `remote_player_handles_iter()` — iterate over remote players without allocation
  - `spectator_handles_iter()` — iterate over spectators without allocation
  - `all_player_handles_iter()` — iterate over all handles without allocation
  - `handles_by_address_iter(addr)` — iterate over handles by address without allocation
- Zero-allocation iterator method for `SyncTestSession`:
  - `local_player_handles_iter()` — iterate over local players without allocation
- `PlayerRegistry` convenience methods for player type queries:
  - `is_local_player(handle)` — check if handle is a local player
  - `is_remote_player(handle)` — check if handle is a remote player
  - `is_spectator_handle(handle)` — check if handle is a spectator
  - `player_type(handle)` — get the `PlayerType` for a handle
  - `num_local_players()` — count of local players
  - `num_remote_players()` — count of remote players (excluding spectators)
  - `all_player_handles()` — all registered handles
  - `remote_player_handle_required()` — returns error if not exactly 1 remote player
- `P2PSession` convenience methods for 1-local-player games:
  - `local_player_handle()` — first local player handle (returns `Option`)
  - `local_player_handle_required()` — returns error if not exactly 1 local player
  - `remote_player_handle()` — first remote player handle
  - `remote_player_handle_required()` — returns error if not exactly 1 remote player
  - `is_local_player(handle)` — check if handle is a local player
  - `is_remote_player(handle)` — check if handle is a remote player
  - `is_spectator_handle(handle)` — check if handle is a spectator
  - `player_type(handle)` — get the `PlayerType` for a handle
  - `num_local_players()` — count of local players
  - `num_remote_players()` — count of remote players
  - `all_player_handles()` — all registered handles
- `SyncTestSession` convenience methods:
  - `local_player_handles()` — all player handles (all are local in sync test)
  - `local_player_handle()` — first local player handle (returns `Option`)
  - `local_player_handle_required()` — returns error if not exactly 1 player
- `Display` impl for core types: `Frame`, `PlayerHandle`, `DesyncDetection`, `PlayerType`, `SessionState`, `InputStatus`, `FortressEvent`, `FortressRequest` — enables human-readable formatting for logging and debugging
- `Display` impl for configuration types: `SyncConfig`, `ProtocolConfig`, `SpectatorConfig`, `InputQueueConfig`, `TimeSyncConfig`, `SaveMode` — enables configuration summary output
- `Display` impl for network types: `NetworkStats`, `ConnectionStatus`, `ProtocolState`, `Event`, `ChaosConfig`, `ChaosStats` — enables network diagnostics logging
- `Display` impl for sync types: `SyncHealth` — enables sync status display
- `Display` impl for prediction strategies: `RepeatLastConfirmed`, `BlankPrediction` — enables strategy identification in logs
- `Display` impl for error types: `FortressError`, `IndexOutOfBounds`, `InvalidFrameReason`, `RleDecodeReason`, `DeltaDecodeReason`, `InternalErrorKind`, `InvalidRequestKind`, `SerializationErrorKind`, `SocketErrorKind` — enables structured error output
- `Display` impl for checksum types: `ChecksumAlgorithm`, `ChecksumError` — enables checksum diagnostics
- `Display` impl for telemetry types: `ViolationSeverity`, `ViolationKind`, `SpecViolation`, `InvariantViolation` — enables telemetry output

### Changed

- **Breaking:** `PlayerRegistry::local_player_handles()` now returns `HandleVec` instead of `Vec<PlayerHandle>`. `HandleVec` implements `Deref<Target = [PlayerHandle]>`, so most code using `.iter()`, `.len()`, or slice operations works unchanged. Use `.to_vec()` if you need a `Vec`.
- **Breaking:** `PlayerRegistry::remote_player_handles()` now returns `HandleVec` instead of `Vec<PlayerHandle>`
- **Breaking:** `PlayerRegistry::spectator_handles()` now returns `HandleVec` instead of `Vec<PlayerHandle>`
- **Breaking:** `PlayerRegistry::all_player_handles()` now returns `HandleVec` instead of `Vec<PlayerHandle>`
- **Breaking:** `PlayerRegistry::handles_by_address()` now returns `HandleVec` instead of `Vec<PlayerHandle>`
- **Breaking:** `P2PSession::local_player_handles()` now returns `HandleVec` instead of `Vec<PlayerHandle>`
- **Breaking:** `P2PSession::remote_player_handles()` now returns `HandleVec` instead of `Vec<PlayerHandle>`
- **Breaking:** `P2PSession::spectator_handles()` now returns `HandleVec` instead of `Vec<PlayerHandle>`
- **Breaking:** `P2PSession::all_player_handles()` now returns `HandleVec` instead of `Vec<PlayerHandle>`
- **Breaking:** `P2PSession::handles_by_address()` now returns `HandleVec` instead of `Vec<PlayerHandle>`
- **Breaking:** `SyncTestSession::local_player_handles()` now returns `HandleVec` instead of `Vec<PlayerHandle>`
- **Breaking:** Added `InvalidRequestKind::NoLocalPlayers` variant — exhaustive matches on `InvalidRequestKind` must now handle this case
- **Breaking:** Added `InvalidRequestKind::MultipleLocalPlayers` variant — exhaustive matches on `InvalidRequestKind` must now handle this case
- **Breaking:** Added `InvalidRequestKind::NoRemotePlayers` variant — exhaustive matches on `InvalidRequestKind` must now handle this case
- **Breaking:** Added `InvalidRequestKind::MultipleRemotePlayers` variant — exhaustive matches on `InvalidRequestKind` must now handle this case
- **Breaking:** `P2PSession::is_spectator()` renamed to `is_spectator_handle()` for consistency with `PlayerRegistry`. Update calls from `session.is_spectator(handle)` to `session.is_spectator_handle(handle)`.
- Optimized convenience methods `local_player_handle()`, `remote_player_handle()`, `local_player_handle_required()`, and `remote_player_handle_required()` to use iterators directly, avoiding temporary allocations

## [0.4.1]

- **Breaking:** `PlayerHandle` Display format changed from raw index (`0`) to labeled format (`PlayerHandle(0)`) for clearer log output. Update any code that parses `PlayerHandle` Display output.

## [0.4.0] - 2026-01-30

### Added

- `GameStateCell::load_or_err()` method for strict state loading with proper error handling
- `SessionBuilder::with_lan_defaults()` preset for low-latency LAN play
- `SessionBuilder::with_internet_defaults()` preset for typical online play
- `SessionBuilder::with_high_latency_defaults()` preset for mobile/unstable connections
- `Frame` ergonomic methods for safe arithmetic and conversion:
  - `as_usize()`, `try_as_usize()` — convert to usize with Option/Result
  - `buffer_index(size)`, `try_buffer_index(size)` — ring buffer index calculation
  - `try_add(i32)`, `try_sub(i32)` — Result-returning arithmetic
  - `next()`, `prev()` — Result-returning increment/decrement
  - `saturating_next()`, `saturating_prev()` — saturating increment/decrement
  - `from_usize(usize)`, `try_from_usize(usize)` — safe construction from usize
  - `distance_to(Frame)` — signed distance calculation
  - `is_within(window, reference)` — window proximity check
- `Debug` impl for `P2PSession`, `SpectatorSession`, and `SyncTestSession` — enables logging session state for debugging
- `Debug` impl for `ChaosSocket` — shows config, stats, and packet queue length
- `Debug` impl for `GameStateAccessor` — delegates to inner `T` when `T: Debug`
- `PartialEq` derive for `ChaosConfig` — enables configuration comparison in tests
- `Hash` derive for `ChaosStats`, `NetworkStats`, and `Pcg32` — enables use as map keys
- `Copy`, `PartialEq`, `Eq`, and `Hash` derives for `TracingObserver` unit struct
- `Hash` derive for configuration types: `TimeSyncConfig`, `SyncConfig`, `ProtocolConfig`, `SpectatorConfig`, `InputQueueConfig` — enables use as map keys for configuration caching
- `PartialEq`, `Eq`, and `Hash` derives for `DeterministicHasher` and `DeterministicBuildHasher` — enables comparison and use as map keys

### Changed

- **Breaking:** Added `InvalidFrameReason::MissingState` variant — exhaustive matches on `InvalidFrameReason` must now handle this case
- **Breaking:** Added `FortressError::FrameArithmeticOverflow` variant — exhaustive matches on `FortressError` must now handle this case
- **Breaking:** Added `FortressError::FrameValueTooLarge` variant — exhaustive matches on `FortressError` must now handle this case
- **Breaking:** Added `InvalidRequestKind::ZeroBufferSize` variant — exhaustive matches on `InvalidRequestKind` must now handle this case

## [0.3.0] - 2026-01-28

### Added

- `SessionBuilder::add_local_player()` convenience method for adding local players
- `SessionBuilder::add_remote_player()` convenience method for adding remote players
- `P2PSession::local_player_handle()` for easily getting the first local player handle
- `ProtocolConfig` now re-exported in `fortress_rollback::prelude`
- `sync_test` example demonstrating `SyncTestSession` determinism verification
- `request_handling` example demonstrating both manual matching and the `handle_requests!` macro
- Structured error reason types for zero-allocation error construction and programmatic inspection:
  - `IndexOutOfBounds` struct for out-of-bounds errors with collection name, index, and length
  - `InvalidFrameReason`, `RleDecodeReason`, `DeltaDecodeReason` enums for specific failure modes
  - `InternalErrorKind`, `InvalidRequestKind`, `SerializationErrorKind`, `SocketErrorKind` enums
- New `FortressError` variants using structured types: `InvalidFrameStructured`, `InternalErrorStructured`, `InvalidRequestStructured`, `SerializationErrorStructured`, `SocketErrorStructured`
- `ChecksumAlgorithm` and `CodecOperation` enums for identifying operations in errors
- `CompressionError` enum for RLE and delta decode errors

### Changed

- **Breaking:** Removed `#[non_exhaustive]` from `FortressError`, `FortressEvent`, `FortressRequest`, `ViolationKind`, `CompressionError`, `CodecOperation`, `CodecError`, `ChecksumError`, `ChecksumAlgorithm`, `InvalidFrameReason`, `RleDecodeReason`, `DeltaDecodeReason`, `InternalErrorKind`, `InvalidRequestKind`, `SerializationErrorKind`, and `SocketErrorKind` — users can now write exhaustive matches without wildcard arms
- **Breaking:** `ChecksumError::SerializationFailed` now uses struct fields `{ algorithm, message }` instead of tuple
- **Breaking:** `CodecError::EncodeError` and `DecodeError` now use struct fields `{ message, operation }` instead of tuple

## [0.2.2] - 2026-01-22

### Changed

- **Breaking:** Renamed `Result` type alias to `FortressResult` to avoid shadowing `std::result::Result` when using glob imports (`use fortress_rollback::*`)

### Fixed

- Removed the possibility for an internal panic under debug builds

## [0.2.1] - 2025-12-26

### Added

- `ProtocolConfig::deterministic(seed)` preset for fully reproducible network sessions
- `ProtocolConfig::protocol_rng_seed` field for deterministic RNG seeding
- `SessionBuilder::with_event_queue_size()` for configurable event queue capacity
- `ProtocolConfig::input_history_multiplier` field with presets (competitive=2, high_latency=3)
- `ProtocolConfig::validate()` method for configuration validation
- `#[must_use]` attributes on key session methods (`advance_frame()`, `disconnect_player()`, etc.)

### Changed

- **Breaking:** `SessionBuilder::with_input_delay()` and `with_num_players()` now return `Result<Self, FortressError>` instead of silently clamping invalid values
- Replaced floating-point arithmetic with integer-only calculation in `TimeSync::average_frame_advantage()` to eliminate potential non-determinism
- Replaced `DefaultHasher` with `DeterministicHasher` (FNV-1a) in `timing_entropy_seed()` for cross-platform consistency
- Reduced cloning overhead in `poll_remote_clients()` by using `Arc<[PlayerHandle]>` instead of `Vec<PlayerHandle>`
- Pre-allocated compression buffers to reduce allocations in network hot paths

### Fixed

- Fixed sync timeout event flooding that could occur under certain conditions

## [0.2.0] - 2025-12-20

### Added

- Added optional `json` feature for JSON serialization of telemetry types
  - Provides `to_json()` and `to_json_pretty()` methods on `SpecViolation` and `InvariantViolation`
  - Enable with `features = ["json"]` in Cargo.toml
- Added `SyncConfig::extreme()` preset for very hostile network conditions
  - Sends 20 sync packets (vs 10 for mobile, 5 for default) with 250ms retry intervals
  - 30-second sync timeout to handle multiple burst loss events
  - Designed for scenarios with 10%+ burst loss probability and 8+ packet burst lengths
  - Not recommended for production use due to long timeouts
- Added burst loss recommendations to packet loss documentation table

### Changed

- **Breaking:** `serde_json` is now an optional dependency behind the `json` feature
  - Reduces default dependency count from 7 to 6 production dependencies
  - Users who need `to_json()` methods must enable the `json` feature
  - The telemetry types still implement `serde::Serialize` for use with any serializer
- Restructured test code to further reduce published crate size
  - Moved network peer binary to separate `tests/network-peer` crate
  - Excluded additional test infrastructure from published package

### Fixed

- Fixed flaky CI tests on macOS under high load by using more robust timing margins
- Improved test reliability for high packet loss scenarios using appropriate sync presets

## [0.1.2] - 2025-12-19

### Changed

- Reduced published crate size from 2.43 MiB to ~300 KiB (88% reduction)
  - Excluded `actionlint` binary that was accidentally committed
  - Excluded `tests/` directory (users can clone repo to run tests)
  - Excluded `Cargo.lock` (not needed for library crates)
  - Excluded development config files (`.cargo/`, `.config/`, `clippy.toml`, etc.)
  - Excluded LLM instruction files (`AGENTS.md`, `CLAUDE.md`, `.llm/`)
  - Excluded `supply-chain/` cargo-vet metadata

## [0.1.1] - 2025-12-19

### Changed

- Reduced crate size by excluding irrelevant artifacts from published package

## [0.1.0] - 2025-12-19

Initial release of Fortress Rollback, a correctness-first fork of GGRS v0.11.0.

### Added

#### Desync Detection API

- `SyncHealth` enum for synchronization status reporting (`InSync`, `Pending`, `DesyncDetected`)
- `P2PSession::sync_health(peer)` to query synchronization status with a specific peer
- `P2PSession::is_synchronized()` to check if all peers are in sync
- `P2PSession::all_sync_health()` to get sync status for all remote peers
- `P2PSession::last_verified_frame()` to get the highest frame with successful checksum verification
- `NetworkStats` fields for desync monitoring: `last_compared_frame`, `local_checksum`, `remote_checksum`, `checksums_match`
- `InvariantChecker` implementation for `P2PSession` to validate session health

#### Deterministic Hashing

- `fortress_rollback::hash` module with deterministic FNV-1a hashing utilities
- `DeterministicHasher` for consistent cross-process checksums
- `fnv1a_hash` convenience function for computing deterministic hashes
- `DeterministicBuildHasher` for use with collections requiring deterministic hashing

#### Configuration APIs

- Structured configuration: `SyncConfig`, `ProtocolConfig`, `TimeSyncConfig`, `SpectatorConfig`, `InputQueueConfig`
- Preset-based configuration methods (e.g., `SyncConfig::high_latency()`, `ProtocolConfig::competitive()`)
- `SaveMode` enum replacing deprecated `with_sparse_saving_mode()`
- `ViolationObserver` trait and `CollectingObserver` for monitoring internal invariant violations

#### Session APIs

- `P2PSession::confirmed_inputs_for_frame(frame)` for computing deterministic checksums over confirmed state
- `InputQueue` now tracks player index for player-specific prediction strategies

#### Development Infrastructure

- Pre-commit hooks configuration for code quality automation (markdownlint, link validation, cargo fmt/clippy)
- `docs.yml` CI workflow for documentation and link validation
- `scripts/docs/check-links.sh` for local file reference validation
- Comprehensive test suite: 1100+ library and integration tests, multi-process network tests passing
- TLA+ `Concurrency.tla` specification for `GameStateCell` thread safety verification

### Changed

#### Determinism Improvements

- Replaced all `HashMap` with `BTreeMap` for guaranteed iteration order
- Replaced all `HashSet` with `BTreeSet` for deterministic iteration
- `Config::Address` now requires `Ord` + `PartialOrd` trait bounds (see Migration section)
- Test infrastructure uses `fnv1a_hash` instead of `DefaultHasher`

#### Default Behavior

- **Desync detection now enabled by default**: `DesyncDetection::On { interval: 60 }` (once per second at 60fps). This catches state divergence early. Users needing to disable detection can explicitly set `DesyncDetection::Off`.

#### Branding

- Crate renamed to `fortress-rollback` (import as `fortress_rollback`)
- All `Ggrs*` types renamed to `Fortress*`:
  - `GgrsError` → `FortressError`
  - `GgrsEvent<T>` → `FortressEvent<T>`
  - `GgrsRequest<T>` → `FortressRequest<T>`
- All documentation updated to reference "Fortress Rollback"

#### Performance Improvements

- `FortressRequest::AdvanceFrame { inputs }` now uses `InputVec<T::Input>` (a `SmallVec<[(T::Input, InputStatus); 4]>`) instead of `Vec`. This avoids heap allocations for games with 1-4 players.
- `synchronized_inputs()` returns `InputVec` for stack-allocated inputs in the common case

#### Safety Improvements

- `InputQueue::confirmed_input` now returns `Result` instead of panicking
- Spectator confirmed-input path bubbles `FortressError` on missing data

### Fixed

- Crash when misprediction detected at frame 0 (first frame): `adjust_gamestate()` no longer attempts to load the current frame when receiving early corrections
- Multi-process rollback desync (BUG-001): Window-based checksum computation using last 64 frames ensures frames are always available for comparison between peers
- `PlayerRegistry::spectator_handles()` incorrectly returned local player handles in addition to spectators
- All 35 multi-process network tests now pass reliably

### Removed

- Historical GGRS changelog entries moved to [docs/ggrs-changelog-archive.md](docs/ggrs-changelog-archive.md)

---

## Breaking Changes from GGRS

This section summarizes breaking changes for users migrating from GGRS v0.11.0.

### Dependency Change

```toml
# Before
[dependencies]
ggrs = "0.11"

# After
[dependencies]
fortress-rollback = "0.9"
```

### Import Path Change

```text
// Before
use ggrs::{SessionBuilder, P2PSession, GgrsError};

// After
use fortress_rollback::{SessionBuilder, P2PSession, FortressError};
```

### Type Renames

| Old Name         | New Name             |
| ---------------- | -------------------- |
| `GgrsError`      | `FortressError`      |
| `GgrsEvent<T>`   | `FortressEvent<T>`   |
| `GgrsRequest<T>` | `FortressRequest<T>` |

### Address Trait Bounds

`Config::Address` now requires `Ord` + `PartialOrd`:

```text
// Before
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct MyAddress { /* ... */ }

// After
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct MyAddress { /* ... */ }
```

### Input Vector Type

The `inputs` field in `FortressRequest::AdvanceFrame` is now `InputVec<T::Input>` (a `SmallVec`) instead of `Vec`:

```text
// If you have explicit type annotations:
// Before
fn handle_inputs(inputs: Vec<(MyInput, InputStatus)>) { ... }

// After
use fortress_rollback::InputVec;
fn handle_inputs(inputs: InputVec<MyInput>) { ... }
// Or accept a slice for flexibility:
fn handle_inputs(inputs: &[(MyInput, InputStatus)]) { ... }
```

### Behavioral Notes

- Session termination using `confirmed_frame()` alone is incorrect; use the new `SyncHealth` API for proper synchronization verification. See [docs/migration.md](docs/migration.md) for details.
- **Desync detection is now enabled by default** (`DesyncDetection::On { interval: 60 }`). GGRS defaulted to `Off`. Explicitly set `DesyncDetection::Off` if you need the old behavior.

---

For detailed migration instructions, see [docs/migration.md](docs/migration.md).

[Unreleased]: https://github.com/wallstop/fortress-rollback/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/wallstop/fortress-rollback/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/wallstop/fortress-rollback/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/wallstop/fortress-rollback/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/wallstop/fortress-rollback/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/wallstop/fortress-rollback/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/wallstop/fortress-rollback/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/wallstop/fortress-rollback/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/wallstop/fortress-rollback/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/wallstop/fortress-rollback/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/wallstop/fortress-rollback/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/wallstop/fortress-rollback/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/wallstop/fortress-rollback/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/wallstop/fortress-rollback/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/wallstop/fortress-rollback/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/wallstop/fortress-rollback/releases/tag/v0.1.0
