<!-- SYNC: This source doc syncs to wiki/Reconnect-Playbook.md. -->

# Disconnect and Rejoin Playbook

Fortress Rollback distinguishes a temporary interruption, resumed traffic, a committed peer
drop, and a terminal disconnect. Handle these events separately; they are not interchangeable.

## Event handling

| Event | Meaning | Application action |
| --- | --- | --- |
| `NetworkInterrupted` | No traffic arrived for the notification delay; the disconnect timeout has not fired | Show degraded connectivity, keep polling, and avoid declaring the peer gone |
| `NetworkResumed` | Traffic resumed after an interruption | Clear the warning and keep the same player/session identity |
| `PeerDropped` | A graceful-drop certificate committed for a player handle | Apply AI takeover, removal, pause, or match rules for that slot |
| `Disconnected` | The endpoint disconnected or a graceful drop completed | Close endpoint UI/resources; inspect policy and companion events before deciding match outcome |

Always drain the entire `events()` batch. A graceful removal emits `PeerDropped` followed by
`Disconnected`; reacting to only the latter loses the player-handle context.

## Choosing drop behavior

`DisconnectBehavior::Halt` is the compatibility default. Once a remote drops, confirmed progress
stops. Use it for matches where any disconnect ends the round.

`DisconnectBehavior::ContinueWithout` makes automatic timeout use the coordinated graceful-drop
flow. Survivors hold confirmation while they agree on the retained prefix, then freeze the
dropped slot and continue. Use `remove_player(handle)` for an application-initiated graceful
leave, kick, or surrender. The legacy `disconnect_player(handle)` always uses halt semantics;
choosing `ContinueWithout` does not make that call graceful.

Every peer should use the same policy and lifecycle entry point. A partition that cannot produce
the required certificate fails closed instead of inventing a new confirmed history.

## Timeout presets

Start from measured conditions and the presets in the
[user guide](user-guide.md#network-scenario-configuration-guide).
The notification delay should tolerate ordinary jitter; the timeout should tolerate the longest
supported loss burst plus processing/scheduling overhead. Test both under packet loss and
application stalls. Browser background throttling needs its own test row.

## Hot-join rejoin flow

Hot join is feature-gated and is not transparent reconnection:

1. Configure the original mesh for hot join and graceful peer removal.
2. Complete the old generation's coordinated drop. Do not reuse a live slot prematurely.
3. Build the replacement with `start_hot_join_session` using input delay zero, a prediction
   window of at least one, and the save-mode requirements documented by the builder.
4. Keep polling every survivor and the joiner through snapshot transfer and activation.
5. Treat the joiner as active only when its session reaches `Running`.
6. Inspect `hot_join_metrics()`: `completed` must be true, while `polls_to_running` and
   `millis_to_running` provide the observed activation cost. Alert against a deployment-specific
   bound established by soak data, not a universal hard-coded duration.

If activation times out or a survivor fails closed, preserve the old confirmed prefix and collect
the same diagnostic bundle described in the [desync playbook](desync-playbook.md). Do not force a
local-only slot activation.
