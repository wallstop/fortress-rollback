<!-- SYNC: This source doc syncs to wiki/Tuning.md. -->

# Network Tuning Guide

Tune from measurements, not labels such as “Wi-Fi” or “mobile.” Input delay trades local response
latency for fewer/deeper rollbacks; the prediction window trades short outage tolerance for more
speculative work. Increasing either does not repair nondeterministic game logic.

## Consistency and availability contract

**Consistency** means that peers in one session agree on the ordered inputs and deterministic game
state for every frame in their shared confirmed prefix. Speculative frames may differ and roll back;
that responsiveness is a separate property, measured by visible frame progress, rollback frequency,
and correction depth. **Confirmed availability** is also separate: measure confirmed-frame progress,
confirmation lag, and stall duration rather than inferring them from speculative rendering.

These guarantees assume a deterministic game, fixed-width network inputs, matching session and
lifecycle configuration, and an application that polls the network and drains events even while
obeying `WaitRecommendation`. The simulation harness supplies one poll opportunity per 16 ms virtual
step. Recovery measurements also require delivery to resume after the injected fault heals; a
permanent partition has no recovery-time bound.

The CAP boundary is explicit: a permanent partition cannot both advance confirmed play in every
component and preserve one history. Fortress must preserve the held confirmed prefix and stop or fail
closed until healing or a valid coordinated drop establishes one successor history. Speculative
progress during a partition is not evidence of confirmed availability. See the
[disconnect and rejoin playbook](reconnect-playbook.md) for the application-facing lifecycle.

## Measured baseline

The rows below use schema v3. At commit `8c8660b`, all ten PR-gate cells matched
`tests/simulation/baselines/sweep-v3.json` bit-for-bit (1,000 steps, seed 1), and the release-mode
full matrix passed all 136 cells (5,000 steps, seed 1) with zero desyncs and no oracle failures. The 16-player row comes
from that current full matrix. Sessions are configured for 60 FPS; the harness advances once every
16 ms, providing 62.5 update opportunities per virtual second. Runs use input delay 1, max prediction
8, and deterministic virtual time. Encoded payload demand is measured when Fortress enqueues
messages and excludes IP/UDP headers; it is not observed transport throughput. Schema v3 also records
protocol messages enqueued for eventual socket submission per player per second. The clean
two-player row is 135.8125, including one-time synchronization traffic.

The checked-in gate covers constant loss, RTT, and jitter for the listed two- and four-player cells;
the full matrix adds the 3-, 8-, 12-, and 16-player regional rows. The partition, duplication,
reordering, burst-loss, and asymmetry evidence below is separate. Repeated flaps and poll stalls
still need reproducible rows with nonzero injection counters before their consistency claims can expand.

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

## Partition and directed-fault consistency evidence

At commit `726b25f`, a separate, expectation-aware schema-v5 ledger records 52 fixed-budget rows using
900 virtual steps, 4-byte inputs, and a materialized schedule at N=2/3/4/8/16. The 40 rows without
state/checksum corruption contain 25 bounded recoveries—five each for short-heal partitions, directed
duplication, reordering, burst loss, and asymmetric delay—and 15 safe failures: long-heal terminal
`Halt`, permanent `Halt`, and permanent uncertified `ContinueWithout`. Every intended partition link
dropped traffic. Each directed fault window affected only link 0→1, with nonzero per-link counters
matching the mesh-global total. Reordering uses deterministic jitter and counts a copy only when it
arrives after a later-scheduled copy on the same link. Burst loss uses a seeded 0.05 per-send start
probability and length three; every row observed eight starts and an actual three-input loss run.

Asymmetric delay applies 200 ms only to 0→1 from step 100 through step 180; the reverse direction stays
clean. The matrix counted 170 delayed copies at N=2 and 132 at each larger size. At healing, new clean
sends legitimately overtake old delayed traffic, producing 26 directed reorder observations at N=2 and
22 at each larger size. Independent timing, receiver-order, payload-loss, and delayed-duplication tests
validate direction, virtual-time boundaries, and per-copy instrumentation. All 52 rows match their
complete replay evidence, not only their trace hashes.

Twelve matched corruption controls classify as defects with typed state/checksum, peer, and frame
evidence. The directed-fault controls at N=2 and N=16 inject state corruption at frame 120, inside the
fault window, and bound detection by the confirmed prefix at healing. The 15 safe-failure rows each
retain a 64-step (1.024-second) terminal plateau after a useful pre-partition confirmed prefix. This
finite observation is not a proof of permanent liveness. Odd permanent-`ContinueWithout` rows isolate
every peer; two-component majority/minority progress needs the component-scoped state oracle tracked
in `PLAN.md`.

Reproduce the partition and directed-fault artifact with:

```bash
FORTRESS_SWEEP_GIT_SHA=726b25f47fb51213c71a8ebcbb47b07dfa7757f1 \
FORTRESS_PARTITION_SWEEP_OUT=/tmp/partition-matrix.jsonl \
cargo nextest run --profile ci-network-nightly --release \
  --run-ignored ignored-only -E 'test(partition_sweep_full)' \
  --test simulation --no-capture
```

The nightly `deterministic-sweep-nightly-results` artifact includes this ledger as
`partition-matrix.jsonl`. It records sync steps, recovery anchors, confirmation and rollback metrics,
wire totals, queue high-water marks, and per-link injection evidence. Host resource use and exact
recovery/stall durations remain unmeasured rather than zero-filled.

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
   confirmation lag, `pending_output_len`, exact bytes and packets per second, plus the adapter's
   downstream service rate, queue depth, and oldest-message age.
4. Raise input delay when rollback work is the problem. Raise max prediction only when short
   gaps stall an otherwise healthy session and the game can afford the extra speculative work.
5. Test the same configuration at N=2 and the max supported player count. Full-mesh traffic
   and confirmation work grow with peer count.
6. Bless a new ledger only after explaining every material change.

`WaitRecommendation` is a runtime correction signal, not a replacement for tuning. Continue
network polling while obeying it so slowing simulation does not also delay packet processing.
