<!-- SYNC: This wiki page is generated from docs/tuning.md. Edit docs source. -->

# Network Tuning Guide

Tune from measurements, not labels such as “Wi-Fi” or “mobile.” Input delay trades local response
latency for fewer/deeper rollbacks; the prediction window trades short outage tolerance for more
speculative work. Increasing either does not repair nondeterministic game logic.

## Measured baseline

The 2–4 player rows below are from `tests/simulation/baselines/sweep-v2.json`, schema v2, last
changed at commit `b535d3b`. The 16-player rows come from the same schema's release-mode full
matrix at commit `991559c` (5,000 steps, seed 1). Runs use 60 FPS, input delay 0, max
prediction 8, and deterministic virtual time. Bandwidth is exact Fortress wire payload per player
and excludes IP/UDP headers.

| Players / path | Loss / RTT / jitter | Input | Rollbacks / 100 frames | Rollback p99 | Wire KiB/s/player | Mean / max confirmation lag | Stalls/min |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 2 / LAN | 0% / 10 ms / 0 ms | 4 B | 0.10 | 1 | 4.71 | 1.00 / 2 | 0 |
| 2 / Wi-Fi | 1% / 50 ms / 20 ms | 4 B | 66.65 | 4 | 5.22 | 2.62 / 8 | 0 |
| 2 / mobile | 5% / 100 ms / 20 ms | 4 B | 71.06 | 7 | 5.84 | 4.33 / 8 | 15 |
| 4 / Wi-Fi | 1% / 50 ms / 20 ms | 4 B | 97.23 | 4 | 17.73 | 6.12 / 8 | 69.38 |
| 4 / severe | 15% / 200 ms / 20 ms | 4 B | 119.51 | 6 | 6.88 | 6.69 / 8 | 2,064.38 |
| 16 / regional | 1% / 100 ms / 20 ms | 4 B | 144.91 | 4 | 117.09 | 7.39 / 8 | 1,218.42 |

The 32-byte input rows preserve the same rollback and lag results but cost 8.03, 13.44, 19.72,
41.85, 17.71, and 220.10 KiB/s/player respectively. The N=16 row remained at zero desyncs and
confirmed through frame 3,292, but its stalls and bandwidth are a capacity warning, not a general
endorsement of 16-player full mesh. Keep network inputs fixed-width and as small as the game
permits.

## Starting configurations

These are operational starting points from the
[user-guide presets](User-Guide#network-scenario-configuration-guide),
not claims measured by the baseline above. Re-run the sweep with your input width and game-state
cost before shipping them.

| Measured condition | Input delay | Max prediction | Starting preset | Expected baseline behavior |
| --- | ---: | ---: | --- | --- |
| Stable LAN, RTT under 20 ms | 0–1 | 4–6 | `SyncConfig::lan()` | Near-zero rollback load; confirmation about 1–2 frames behind |
| Typical Wi-Fi, RTT around 50 ms and ~1% loss | 2 | 8 | defaults / `TimeSyncConfig::smooth()` | Rollback p99 around 4 in the zero-delay baseline |
| Mobile, RTT around 100 ms and ~5% loss | 3 | 12–15 | `SyncConfig::lossy()` | Baseline reaches the prediction limit and begins stalling |
| Four-player mixed Wi-Fi | 3 | 12 | defaults / `TimeSyncConfig::smooth()` | Slowest link dominates; watch per-peer queues and lag |
| Transparent relay fallback, +20–80 ms additional RTT | Re-measure | Re-measure | Start from the direct-path preset, then tune against total RTT | Research-derived sensitivity range, not a measured Fortress baseline; include relayed sessions in qualification |
| Sustained 15% loss / 200 ms RTT | Do not silently accept | N/A | matchmaking warning or alternate topology | Default baseline spends much of the run stalled |

The relay fallback row models a transport transparently relaying the same peer-to-peer datagrams;
it is not the future server-star input topology. Its latency range is a research-derived
sensitivity assumption. Measure the direct and relayed populations separately and select input
delay and prediction windows from the relayed path's observed total RTT and receipt-gap tail.

## Frame-advantage measurement limits

Each endpoint estimates the remote frame by aging the last received frame with `RTT/2`. That
assumes symmetric one-way delay: a ping-pong exchange cannot identify how the RTT divides between
directions. Smoothing reduces short-term jitter, but it cannot remove a persistent asymmetric-path
bias. This `RTT/2` packet-age estimate is separate from the controller's later division by two,
which damps the two-sided frame-advantage signal.

The bounded H-ASYM experiment compared constant 10 ms / 200 ms one-way delays against a matched
105 ms / 105 ms control at equal 210 ms RTT (`N=2`, 900 steps). The asymmetric run ended seven
visual frames apart and recorded stalls of 18 versus 11, but produced zero `WaitRecommendation`
events or obeyed skips; endpoint gauges stayed in `[-1, +1]`, below the three-frame dead band.
That row demonstrates transport/prediction asymmetry without the hypothesized chronic pacing
correction. Jitter, `N>2`, and other asymmetry ratios remain outside that result, so qualify them
separately instead of treating timestamp-based one-way estimation as implemented.

## Measurement loop

1. Record RTT, loss bursts, jitter, player count, serialized input width, and simulation cost.
2. Start with the smallest input delay users tolerate and a prediction window covering the p99
   receipt gap.
3. Compare `rollback_count`, the depth histogram, `resimulated_frames`, `stall_count`,
   confirmation lag, `pending_output_len`, and exact bytes per second.
4. Raise input delay when rollback work is the problem. Raise max prediction only when short
   gaps stall an otherwise healthy session and the game can afford the extra speculative work.
5. Test the same configuration at N=2 and the max supported player count. Full-mesh traffic
   and confirmation work grow with peer count.
6. Bless a new ledger only after explaining every material change.

`WaitRecommendation` is a runtime correction signal, not a replacement for tuning. Continue
network polling while obeying it so slowing simulation does not also delay packet processing.
