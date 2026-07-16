<!-- SYNC: This source doc syncs to wiki/Tuning.md. -->

# Network Tuning Guide

Tune from measurements, not labels such as “Wi-Fi” or “mobile.” Input delay trades local response
latency for fewer/deeper rollbacks; the prediction window trades short outage tolerance for more
speculative work. Increasing either does not repair nondeterministic game logic.

## Measured baseline

The 2–4 player rows below are from `tests/simulation/baselines/sweep-v3.json`, schema v3, measured
on base commit `94f33a8`. The 16-player rows come from the prior schema's release-mode full
matrix at commit `991559c` (5,000 steps, seed 1). Sessions are configured for 60 FPS; the harness
advances once every 16 ms, providing 62.5 update opportunities per virtual second. Runs use input
delay 0, max prediction 8, and deterministic virtual time. Encoded payload demand is measured when
Fortress enqueues messages and excludes IP/UDP headers; it is not observed transport throughput.
Schema v3 also records protocol messages enqueued for eventual socket submission per player per
second. The clean two-player row is 135.8125, including one-time synchronization traffic.

| Players / path | Loss / RTT / jitter | Input | Rollbacks / 100 frames | Rollback p99 | Encoded KiB/s/player | Mean / max confirmation lag | Stalls/min |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 2 / LAN | 0% / 10 ms / 0 ms | 4 B | 0.10 | 1 | 4.73 | 1.00 / 2 | 0 |
| 2 / Wi-Fi | 1% / 50 ms / 20 ms | 4 B | 66.65 | 4 | 5.24 | 2.62 / 8 | 0 |
| 2 / mobile | 5% / 100 ms / 20 ms | 4 B | 71.06 | 7 | 5.86 | 4.33 / 8 | 15 |
| 4 / Wi-Fi | 1% / 50 ms / 20 ms | 4 B | 97.23 | 4 | 17.78 | 6.12 / 8 | 69.38 |
| 4 / severe | 15% / 200 ms / 20 ms | 4 B | 119.51 | 6 | 6.95 | 6.69 / 8 | 2,064.38 |
| 16 / regional | 1% / 100 ms / 20 ms | 4 B | 144.91 | 4 | 117.09 | 7.39 / 8 | 1,218.42 |

The 32-byte input rows preserve the same rollback and lag results but cost 8.05, 13.45, 19.73,
41.90, 17.79, and 220.10 KiB/s/player respectively. The N=16 row remained at zero desyncs and
confirmed through frame 3,292, but its stalls and encoded demand are a capacity warning, not a general
endorsement of 16-player full mesh. Keep network inputs fixed-width and as small as the game
permits.

## Starting configurations

These are operational starting points from the
[user-guide presets](user-guide.md#network-scenario-configuration-guide),
not claims measured by the baseline above. Re-run the sweep with your input width and game-state
cost before shipping them.

| Measured condition | Input delay | Max prediction | Starting preset | Expected baseline behavior |
| --- | ---: | ---: | --- | --- |
| Stable LAN, RTT under 20 ms | 0–1 | 4–6 | `SyncConfig::lan()` | Near-zero rollback load; confirmation about 1–2 frames behind |
| Typical Wi-Fi, RTT around 50 ms and ~1% loss | 2 | 8 | defaults / `TimeSyncConfig::smooth()` | Rollback p99 around 4 in the zero-delay baseline |
| Mobile, RTT around 100 ms and ~5% loss | 3 | 12–15 | `SyncConfig::lossy()` | Baseline reaches the prediction limit and begins stalling |
| Four-player mixed Wi-Fi | 3 | 12 | defaults / `TimeSyncConfig::smooth()` | Slowest link dominates; watch per-peer queues and lag |
| Sustained 15% loss / 200 ms RTT | Do not silently accept | N/A | matchmaking warning or alternate topology | Default baseline spends much of the run stalled |

## Measurement loop

1. Record RTT, loss bursts, jitter, player count, serialized input width, and simulation cost.
2. Start with the smallest input delay users tolerate and a prediction window covering the p99
   receipt gap.
3. Compare `rollback_count`, the depth histogram, `resimulated_frames`, `stall_count`,
   confirmation lag, `pending_output_len`, exact bytes and packets per second, plus the adapter's
   downstream service rate, queue depth, and oldest-message age.
4. Raise input delay when rollback work is the problem. Raise max prediction only when short
   gaps stall an otherwise healthy session and the game can afford the extra speculative work.
5. Test the same configuration at N=2 and the max supported player count. Full-mesh traffic
   and confirmation work grow with peer count.
6. Bless a new ledger only after explaining every material change.

`WaitRecommendation` is a runtime correction signal, not a replacement for tuning. Continue
network polling while obeying it so slowing simulation does not also delay packet processing.
