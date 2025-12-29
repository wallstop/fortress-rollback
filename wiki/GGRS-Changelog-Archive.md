# GGRS Historical Changelog Archive

> **Note:** This file contains the historical changelog from the original [GGRS (Good Game Rollback System)](https://github.com/gschup/ggrs) project. Fortress Rollback was forked from GGRS v0.11.0. This archive is preserved for historical context only.
>
> For changes specific to Fortress Rollback, see [changelog.md](Changelog).
> For migration instructions, see [migration.md](Migration).

---

## 0.11.0

- Add `tracing` crate for logging support
- **Breaking change:** `Config::Input` must now satisfy `Default` + serde's `Serialize` & `DeserializeOwned` traits, rather than bytemuck's `Pod` and `Zeroable`
  - This allows enums with fields as well as variable-sized types (such as `Vec`) to be directly used as the GGRS input type, and should generally be more flexible than the old bounds
  - To migrate old code, it's recommended to simply derive `Default` and `Serialize` & `Deserialize` on your `Input` type
  - Or, to migrate old code strictly without changing behavior, implement `Default` in terms of `bytemuck::Zeroable::zeroed()` and implement `Serialize` and `DeserializedOwned` in terms of `bytemuck::bytes_of()` and (probably) `bytemuck::pod_read_unaligned()`
  - Fixes [#40](https://github.com/gschup/ggrs/issues/40) and [#74](https://github.com/gschup/ggrs/issues/74)
- Lockstep determinism is now possible by setting max predictions to 0
- Allow non-`Clone` types to be stored in `GameStateCell`
- Added `SyncTestSession::current_frame()` and `SpectatorSession::current_frame()` to match the existing `P2PSession::current_frame()`
- Added `P2PSession::desync_detection()` to read the session's desync detection mode
- Fix: GGRS no longer panics when a client's local frame advantage exceeds the range of an i8 ([#35](https://github.com/gschup/ggrs/issues/35))
- Fix: GGRS no longer panics when trying to send an overly large UDP packet, unless debug assertions are on
- Fix: GGRS no longer panics when trying to send a message over a custom socket implementation if that message exceeded the maximum safe UDP packet size
- Fix a false positive in `P2PSession`'s desync detection; it was possible for a desync to incorrectly be detected when `P2PSession::advance_frame()` would enqueue a checksum-changing rollback, mark a to-be-rolled-back frame as confirmed, and send that newly-confirmed frame's still-incorrect checksum to peers

## 0.10.2

- Fix dependency versions

## 0.10.1

- SyncTest now checks frames in chronological order

## 0.10.0

- Rename types with GGRS prefix to match Rust naming conventions
- Removed deprecated `GGRSError` variants
- `GameStateCell` now implements Debug
- Fixed a bug where checksums of unconfirmed frames were compared during desync detection
- You can now trigger a desync manually in the example game by pressing SPACE

## 0.9.4

- `SessionBuilder` now implements Debug (requires `Config::Address` to have Debug)
- Optional desync detection for P2P sessions via `with_desync_detection_mode` in the `SessionBuilder`

## 0.9.3

- Added support for fieldless enums in `PlayerInput`

## 0.9.2

- Fixed a bug where sync would not work with RTT higher than SYNC_RETRY_INTERVAL

## 0.9.1

- Fixed multiple local players, added example documentation for it
- Fixed save and advance request ordering during a rollback in P2PSessions

## 0.9.0

- Removed `GameState` from the public API
- Removed `PlayerInput` from the public API; `AdvanceFrame` requests now hand over a tuple with the `InputStatus` and status of that input
- Added `InputStatus` enum to distinguish the status of given inputs
- Users now have to call `add_local_input(..)` for every local player before calling `advance_frame()`
- Enabled multiple players per endpoint
- Sessions are now constructed through a unified `SessionBuilder`
- Overhauled all generics
- Provided inputs are now generic; the user has to only supply a POD struct instead of serialized input
- Added a `Config` trait with types to bundle all generic options
- Renamed `GameInput` to `PlayerInput`
- The user now has to explicitly create a socket themselves before creating a session

## 0.8.0

- `GameState` now is a generic `GameState<T: Clone = Vec<u8>>`, so serialization of game state to save and load is no longer required
- `trait NonBlockingSocket` now is a generic `NonBlockingSocket<A>`, where `A` generalizes the address that the socket uses to send a packet

## 0.7.2

- Massively improved performance by improving input packet handling

## 0.7.1

- Added getter for the max prediction frames parameter in `P2PSession` and `SyncTestSession`

## 0.7.0

- Removed the const `MAX_PREDICTION_FRAMES` and made it a parameter for the user to choose

## 0.6.0

- Added `P2PSession::current_frame()`
- Made `P2PSession::confirmed_frame()` public to let users access it
- Removed the need for a player cap and a maximum input size
- Adjusted session creation API to reflect the above change
- Fixed a bug where a P2P session without remote players would not start
- Migrated to Rust 2021

## 0.5.1

- GGRS no longer panics when packets have been tampered with
- Added `P2PSession::frames_ahead()` that shows how many frames the session believes to be ahead of other sessions

## 0.5.0

- Renamed session constructors to make them more idiomatic; sessions are now created through `P2PSession::new(...)` and `P2PSession::new_with_socket(...)`
- Added functions to create sessions with own sockets provided
- Turned `NonBlockingSocket` into a trait to allow alternate socket types in the future
- Fixed a bug where calling `network_stats` without any time passed would lead to a division by 0
- Fixed a bug where packet transmission time would be accounted for with RTT instead of RTT / 2

## 0.4.4

- Fixed a bug where P2P sessions would falsely skip frames even when able to run the frame
- Implemented some first steps towards WASM compatibility

## 0.4.3

- Changed license from MIT to MIT or Apache 2.0 at the user's option
- Added `local_player_handle()` to `P2PSession`, which returns the handle of the local player
- Added `set_fps(desired_fps)` to `P2PSpectatorSession`

## 0.4.2

- Users are now allowed to save `None` buffers for a `GgrsRequest::SaveRequest`; this allows users to keep their own state history and load/save more efficiently
- Added `num_players()`, `input_size()` getters to all sessions

## 0.4.1

- Added sparse saving feature to `P2PSession`, minimizing the SaveState requests to a bare minimum at the cost of potentially longer rollbacks
- Added `set_sparse_saving()` to `P2PSession` to enable sparse saving
- Added `set_fps(desired_fps)` to `P2PSession` for the user to set expected update frequency; this is helpful for frame synchronization between sessions
- Fixed a bug where a spectator would not handle disconnected players correctly with more than two players
- Fixed a bug where changes to `disconnect_timeout` and `disconnect_notify_start` would change existing endpoints, but would not influence endpoints created afterwards
- Expanded the BoxGame example for up to four players and as many spectators as wanted
- Minor code optimizations

## 0.4.0

- Spectators catch up by advancing the frame twice per `advance_frame(...)` call, if too far behind
- Added `frames_behind_host()` to `P2PSpectatorSession`, allowing to query how many frames the spectator client is behind the last received input
- Added `set_max_frames_behind(desired_value)` to `P2PSpectatorSession`, allowing to set after how many frames behind the spectator fast-forwards to catch up
- Added `set_catchup_speed(desired_value)` to `P2PSpectatorSession`, allowing to set how many frames the spectator catches up per `advance_frame()` call, if too far behind
- In `SyncTestSession`, the user now can (and has to) provide input for all players in order to advance the frame

## 0.3.0

- `GGRSError::InvalidRequest` now has an added `info` field to explain the problem in more detail
- Removed unused `GGRSError::GeneralFailure`
- Removed multiple methods in `SyncTestSession`, as they didn't fulfill any meaningful purpose
- Removed unused sequence number from message header, fixing related issues
- Fixed an issue where out-of-order packets would cause a crash
- Other minor improvements

## 0.2.5

- When a player disconnects, the other players now rollback to that frame; this is done in order to eliminate wrong predictions and resimulate the game with correct disconnection indicators
- Spectators now also handle those disconnections correctly

## 0.2.4

- Fixed an issue where the spectator would assign wrong frames to the input
- Players disconnecting now leads to a rollback to the disconnect frame, so wrongly made predictions can be removed
- In the box game example, disconnected players now spin
- Minor code and documentation cleanups

## 0.2.3

- Fixed an issue where encoding/decoding reference would not match, leading to client desyncs

## 0.2.2

- `SyncTestSession` now actually compares checksums again
- If the user doesn't provide checksums, GGRS computes a fletcher16 checksum
- Internal refactoring/renaming

## 0.2.1

- Fixed an issue where the spectator would only handle one UDP packet and drop the rest

## 0.2.0

- Reworked API: Instead of the user passing a `GGRSInterface` trait object, GGRS now returns a list of `GgrsRequest`s for the user to fulfill

---

*For full GGRS history and original repository, see <https://github.com/gschup/ggrs>*
