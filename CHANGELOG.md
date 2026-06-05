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

### Added

- **Hot Join (reserved-slot model), behind the new opt-in `hot-join` feature flag.** A peer can join a
  running session by filling a *reserved* (or previously gracefully-dropped) player slot: it synchronizes
  with the host, receives a full game-state snapshot, loads it, and resumes normal rollback play — all
  while the total player count (and therefore the network input wire-width) stays fixed, so existing
  peers' delta-compressed input streams are never disturbed.
  - `SessionBuilder::with_hot_join(bool)`, `SessionBuilder::add_reserved_player(addr, handle)` (host side),
    and `SessionBuilder::start_hot_join_session(socket, host_addr)` (joiner side).
  - `SessionState::HotJoining`; `FortressEvent::JoinRequested { handle, addr }` and
    `FortressEvent::PeerJoined { handle, addr }`; `InvalidRequestKind::PlayerCountMismatch { expected, actual }`.
  - Under `hot-join`, `Config::State` additionally requires `Serialize + DeserializeOwned` (for snapshot
    transfer). This bound only applies when the feature is enabled, so it is **not** a breaking change for
    existing builds. Snapshot deserialization is allocation-bounded (a hostile length prefix yields an
    error, never an OOM); keep `Config::State` non-recursive.
  - The join handshake is ack-gated and loss/latency tolerant within a bounded envelope: the host
    re-serves the snapshot until acknowledged and only reactivates the slot on the ack, an abandoned join
    can never stall or fail-close the host (it resumes solo with the slot still reserved), and a joiner
    that loses its snapshot or ack fails cleanly (retryable) rather than desyncing. Scope: 2-peer /
    host-mediated topology; requires `max_prediction >= 1` (lockstep hot-join is rejected at build time).

## [0.9.0] - 2026-06-04

### Added

- Spectator failover via `SessionBuilder::start_spectator_session_multi(&[T::Address], socket)`: a spectator can connect to multiple redundant game peers ("hosts") at once. Unresolved frames use the highest-priority currently connected host by builder order as the canonical source; lower-priority host snapshots stay provisional while a higher-priority host is connected, and if the canonical host disconnects before a frame resolves the next surviving host is promoted only for unresolved frames. Connection status is taken from the selected host's whole-frame snapshot rather than merged player-by-player across hosts. Redundant hosts that disagree on the same player/frame now fail closed: the spectator records a frame-sync violation, emits `FortressEvent::SpectatorDivergence`, and future `advance_frame` calls return `FortressError::SpectatorDivergence` without rewriting frames that already advanced. A clean all-host disconnect with no divergence or malformed canonical data releases the stream-delay boundary so already-buffered frames can drain before `PredictionThreshold`. `SpectatorSession::num_hosts()` reports the current number of connected hosts. The existing single-host `start_spectator_session` delegates to the same construction path and now also rejects invalid `SpectatorConfig` values. If duplicate host addresses are supplied, inbound packets are routed to the first matching host endpoint.
- Enhanced spectator: `SpectatorConfig::stream_delay` holds playback back from the live edge by a configurable number of frames (anti-stream-sniping); the spectator never advances past `last_received_frame - stream_delay`, clamps the delayed boundary at `Frame::NULL` until enough host inputs arrive, and returns `PredictionThreshold` cleanly at the boundary. `SpectatorConfig::enable_rewind` makes the spectator emit a `SaveGameState` each frame and unlocks `SpectatorSession::seek_to_frame(Frame)`, which jumps to any previously simulated frame still in the buffer via a single `LoadGameState` (no re-simulation); seeking on a non-rewind spectator returns `InvalidRequestKind::NotSupported`, a negative target returns `InvalidFrameStructured`/`MustBeNonNegative`, `target + 1` overflow returns `FrameArithmeticOverflow`, and a frame outside the buffered window returns `InvalidFrameStructured`/`MissingState`. New accessors `SpectatorSession::is_rewind_enabled()` and `SpectatorSession::stream_delay()` expose the configured settings. `SpectatorConfig::validate()` rejects only semantic invalid configs (`buffer_size == 0` and `stream_delay >= buffer_size`; `catchup_speed == 0` remains valid), while spectator startup and catch-up request reservation now use fallible allocation so very large user-configured buffers return `None` instead of risking an allocator abort. The spectator advance loop now also returns partial catchup progress instead of discarding already-gathered requests on a mid-batch unavailability; the degenerate `catchup_speed == 0` case (no frame is even attempted while behind) still returns `Ok(<empty>)` rather than `PredictionThreshold`. `seek_to_frame`'s seekable upper bound is `current_frame - 1`: seeking to the exact current frame returns `MissingState` because the post-state for that frame is not saved until the next advance.
- Hardened network input decompression and message decoding against denial of service from hostile length prefixes: protocol receive now derives the accepted RLE decoded length from the user-configured `ProtocolConfig::pending_output_limit` and the reference input size, with `fortress_rollback::rle::DEFAULT_MAX_DECODED_LEN` as a hard upper ceiling; standalone `rle::decode` uses the same public default. Oversized or overflowing decoded lengths return structured errors instead of attempting an unbounded allocation that the default allocator would turn into a process abort. Length accumulation and cursor advancement also use checked arithmetic so crafted packets can no longer overflow `usize`; each run length is range-checked against the limit while still held as `u64` — before any narrowing to `usize` — so the bound cannot be defeated by a 32-bit (`usize == u32`) truncation. Delta decoding also caps the number of decoded frame buffers before allocating, so a tiny reference input cannot fan a bounded byte stream into millions of `Vec` headers; `ProtocolConfig::MAX_PENDING_OUTPUT_LIMIT` keeps valid send batches aligned with that receive cap, and valid senders now split pending input batches to the same byte/frame budget receivers enforce. Built-in UDP, Tokio UDP, and chaos sockets cap each receive poll by raw receive attempts and decoded messages, and grow receive batches fallibly. `network::codec::decode_message()` exposes the same bounded `Message` decoder used by the built-in sockets for custom transports and fuzzing, avoiding generic bincode `Vec` decoding on untrusted peer bytes.
- Network input serialization is now validated as a nonzero, fixed-width byte stream for protocol use. Endpoint creation rejects `Config::Input` types whose default value serializes to zero bytes, rejects local or remote aggregate input frames larger than the receive decode cap, and the sender refuses to queue or delta-encode any per-player input value whose serialized length differs from the default/reference width.
- `ReplayDecodeConfig` and `Replay::from_bytes_with_config()` for callers that want to apply their own encoded replay byte limit or skip post-decode validation. `Replay::from_bytes()` now uses the same checked decoder with the default no-byte-limit config and validates decoded metadata before returning. Every length-prefixed allocation in the decoder (the frame vector, each per-frame input vector, and the checksum vector) is now bounded against the remaining byte slice before any memory is reserved, so a malformed replay cannot drive an out-of-proportion allocation regardless of the configured byte limit.
- `network::compression::decode_with_max_len()` for callers and fuzz targets that need an explicit decoded RLE length policy instead of the default `decode()` policy.
- Configurable socket buffer sizes: `UdpNonBlockingSocket::bind_to_port_with_buffer_sizes` / `from_socket_with_buffer_sizes` and (with the `tokio` feature) `TokioUdpSocket::with_buffer_sizes` / `bind_to_port_with_buffer_sizes` let applications size the reused receive and send buffers for larger serialized inputs. All of these reject a zero buffer size with `io::ErrorKind::InvalidInput` (a zero-length socket buffer can never send or receive) and reserve fallibly, so an over-large size returns an error instead of aborting the process. The infallible `TokioUdpSocket::new` degrades to a smaller buffer under allocator pressure rather than producing a permanently unusable socket, and its receive loop never spins on an empty buffer.
- `SessionBuilder::start_p2p_session` now surfaces the underlying `FortressError` from endpoint creation — the IO/socket, protocol, or configuration cause (including `InvalidRequestKind::AllocationFailed`) — instead of forcing every cause to a single opaque `SerializationErrorKind::EndpointCreationFailed` / `SpectatorEndpointCreationFailed`, so callers can distinguish why synchronization setup failed. `EndpointCreationFailed` is still returned for the specific input-serialization failure that produces it; the previously catch-all `SpectatorEndpointCreationFailed` mapping is removed but the variant remains for backward compatibility.

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

## [0.8.1] - 2026-05-16

### Added

- `P2PSession::set_input_delay()` and `P2PSession::input_delay()` for runtime input-delay adjustment, enabling hybrid delay+rollback in response to changing network conditions. Mid-session **increases** are supported on peers with a single local player: the input queue replicates the most recently added input across the new gap, and the same replicated frames are pushed onto every remote endpoint's pending-output buffer so the remote peer's input sequence remains strictly monotonic. Mid-session **decreases** return an `InputDelayDecreaseUnsupported` error and leave the queue unchanged; mid-session increases on peers with multiple local players return `InputDelayMidSessionMultiLocalUnsupported`. If mid-session increase gap-fill fails an internal queue invariant, the input queue restores its prior state and delay before returning `InputQueueGapFillFailed`. The mid-session gap-fill mirror step now surfaces `InternalErrorStructured(ConnectionStatusIndexOutOfBounds { player_handle })` if the matching `local_connect_status` entry is missing rather than silently skipping the update; if this error is returned through the public API, it indicates an internal-invariant violation and should be treated as a library bug. On a **frozen** input queue (a dropped peer under `ContinueWithout`), `set_input_delay` is unconditionally a silent no-op — including when the requested delay would exceed `max_frame_delay()`, which no longer leaks a `FrameDelayTooLarge` error for an already-gone peer.
- `DisconnectBehavior` enum (`Halt`, `ContinueWithout`) controlling how a `P2PSession` reacts when a remote peer's **automatic** disconnect-timeout fires or when disconnect knowledge is propagated by another peer. `Halt` (default) preserves the legacy GGRS-style halt-on-drop semantics; `ContinueWithout` enables graceful peer drop where remaining peers continue advancing while the dropped peer's input queue is frozen at their last confirmed input. This setting governs only automatic/protocol-observed disconnect paths; explicit `P2PSession::disconnect_player` always preserves halt semantics, and explicit `P2PSession::remove_player` always performs a graceful drop, regardless of this setting. Telemetry note: in the `Event::Disconnected` `ContinueWithout` auto-drop path, `freeze_player` failures are reported with `ViolationSeverity::Error`, the endpoint is still marked disconnected, and the session fails closed by returning to `Synchronizing`. The same fail-closed transition now also fires when applying a propagated disconnect (`update_player_disconnects`) or a direct `Event::Disconnected` returns an internal error before disconnect bookkeeping completes — preventing the session from continuing to advance frames after a disconnect observation has been lost.
- `SessionBuilder::with_disconnect_behavior()` to opt in to graceful peer drop on a `P2PSession`.
- `P2PSession::remove_player()` for explicit graceful removal of a remote peer. Marks every non-spectator player handle owned by the dropped endpoint as disconnected and freezes each one's input queue (so remaining peers see each handle's last confirmed input forever, with `InputStatus::Disconnected`) — multi-handle endpoints (multiple players sharing a single remote address) are handled in full. Disconnects the network endpoint and emits one `FortressEvent::PeerDropped` per non-spectator handle followed by exactly one address-level `FortressEvent::Disconnected` in the same batch. The call is transactional: if the freeze step cannot succeed for every handle, no state-mutating work is performed. Internal-invariant `freeze_player` failures (pre-validated via `SyncLayer::validate_freeze_player`) are surfaced via `Result::Err` (`InternalErrorStructured` with `IndexOutOfBounds` or `InvalidPlayerHandle`) rather than silently returning `Ok(())` with a `ViolationSeverity::Warning`; reaching this branch indicates a library bug. Always opts in to graceful-drop semantics regardless of the configured `DisconnectBehavior` (which only governs the **automatic** disconnect-timeout path). Distinct from the legacy `disconnect_player()`, which preserves the halt-on-drop semantics.
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

> _Follow-up:_ a session-level telemetry hook for input-delay changes (e.g.,
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
