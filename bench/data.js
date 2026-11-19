window.BENCHMARK_DATA = {
  "lastUpdate": 1795074965901,
  "repoUrl": "https://github.com/Ambiguous-Interactive/fortress-rollback",
  "entries": {
    "Fortress Rollback Benchmarks": [
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "de1cdf9d09c9e16e45eaf95c2170f0ca26014c41",
          "message": "test(transport): complete real-path bounds and scope ack exposure (#392)\n\n## Summary\n\nFinishes the active P1 transport slice and scopes the #388 follow-up. No\nprotocol or wire behavior changes.\n\n### Transport completion\n\n- `SocketMetrics` gains `queue_depth` and `oldest_queued_ms`. Built-in\nUDP/Tokio report zero (direct\n  submit); `ChaosSocket` reports its held-packet queue.\n- New real-loopback sampler: 100 datagrams through a seeded 25%\n`ChaosSocket` send-loss path.\nAccepted sends + adapter drops equal 100, delivered traffic is a\npositive subset of accepted, and\nthe loop's decoded count matches the receiver adapter counter. This\nrecords actual adapter\n  acceptance/loss instead of inferring from enqueue demand.\n- New deterministic 150-step (2.4 s) peer-suspension row under an\nexplicit 5 s disconnect timeout.\nBoth peers resume, confirm >500 frames, stay within the prediction\nwindow, and replay exactly.\n- New queue-age test: a 200 ms shaping queue reports depth 1/age 0,\ndepth 1/age 75 ms after 75 ms of\n  virtual time, and depth 0/age 0 after delivery.\n- Existing evidence retained and cited: 256 raw/decoded per-poll cap\nwith next-poll valid recovery;\n128-frame pending-output retransmission boundary and fail-closed beyond;\nbandwidth/MTU/rebind\n  census rows; protocol-valid lie coverage.\n- P1 transport item removed from `PLAN.md`; docs now include a\n\"Transport bound evidence\" section.\n\n### #388 scope\n\nRecorded an explicitly scoped, falsifiable experiment in\n`.agents/skills/design-decisions/references/network.txt` and posted it\non #388:\n\n1. Receipt-only confirmed frame (current bounded split tail).\n2. Always fold `last_acked_input.frame` into `confirmed_frame()`\n(rejected prototype).\n3. Gate only when the unacked span exceeds half the prediction window.\n\nInstrument `last_acked_input.frame`, `confirmed_frame()`, and survivor\nprogress on the #386 one-way\nrow plus `ChaosConfig::terrible_network`; accept only if the cut tail is\n≤1 frame and survivor\nprogress stays within 10% of baseline. The #389 baseline is 6.90/8\nframes mean lag on a lossless\n200 ms 4-peer row, so always-ack-bounding cannot fit the default window.\n\n## Verification\n\n- `cargo nextest run --features tokio,json`: 3,112 passed, 73 skipped, 0\nfailed\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`: clean\n- `agent-preflight.py --auto-fix`, markdownlint, typos,\nchangelog/version/plan hooks: clean\n- Public API census: 2,746 symbols, snapshot matches\n- Diff mutation: the new `ChaosSocket::socket_metrics_impl` mutant is\ncaught by the queue-age test",
          "timestamp": "2026-11-11T00:09:30-08:00",
          "tree_id": "658d76b6b365e28acfd1718e05b63af25bc9dab5",
          "url": "https://github.com/wallstop/fortress-rollback/commit/de1cdf9d09c9e16e45eaf95c2170f0ca26014c41"
        },
        "date": 1794384989333,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 25453,
            "range": "± 451",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 15453,
            "range": "± 886",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 473,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 589,
            "range": "± 87",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 5374,
            "range": "± 173",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2a360dbd46c8bd4a1d6c2f6ae0d450693cc0c1df",
          "message": "test(sim): measure ack-bound exposure and reject both candidates (#393)\n\n## Summary\n\nImplements the scoped #388 experiment end-to-end. No protocol behavior\nchanges: the measured\nresult rejects both acknowledgement-bounded candidates and retains the\nbounded terminal-tail\ncontract.\n\n### Instrumentation\n\n- `UdpProtocol::last_acked_input_frame` exposes the newest acknowledged\nlocal input.\n- `P2PSession::diagnostic_ack_exposure` returns each remote endpoint's\nacknowledged local frame\n  through a doc-hidden diagnostic surface.\n- Simulation harness `RunOptions::ack_exposure_probe_at` captures\ncurrent/confirmed/last-acked per\n  peer at one end-of-step boundary (`RunReport::ack_exposure_probe`).\n- The real `ChaosConfig::terrible_network` arm samples the same gauges\nevery frame on the\n  deterministic virtual clock.\n\n### Measured result\n\n**#386 one-way row** (3 peers, `ContinueWithout`, 900 steps, probe at\ncut+50):\n\n```\nshared=95 tail=9 current=112 confirmed=104 last_acked=95 max_prediction=8\n```\n\n- Receipt-only (current): documented bounded 9-frame tail.\n- Always ack-bound and half-window gating: floor 95, tail 0 — both meet\nthe tail goal.\n\n**Terrible network** (2 peers, 250 ms latency, 15% loss, seeds 42/43):\n\n```\npeak always-bound span = [16, 17] frames, half_gate_active=true, max_prediction=8\n```\n\nBoth candidates would stall survivor progress because their reverse-trip\nfloor exceeds the entire\nprediction window. The rejected prototype's 38/100 frames is the\nexpected consequence.\n\n### Decision\n\nRetain the bounded terminal-tail contract in\n`docs/reconnect-playbook.md` and `docs/tuning.md`.\nRemoving the tail without the liveness cost requires a bounded input\narchive plus a pacing-aware\nexposure policy. The design-decision log records the rejection and this\nprerequisite.\n\n## Tests\n\n- `ack_exposure_candidates_reject_healthy_liveness`: #386 exact replay,\ntail/goal assertions,\nmutated-ack control proving the result depends on the measured ack lag.\n- `terrible_network_ack_exposure_rejects_always_bound`: deterministic\nterrible-network run, exact\n  replay, peak always-bound span and half-window gate assertions.\n- `cargo nextest run --features tokio,json`: 3,114 passed, 73 skipped, 0\nfailed\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`: clean\n- `agent-preflight.py --auto-fix`, markdownlint, typos, version/plan\nhooks: clean\n- Public API census: 2,749 symbols, snapshot matches\n- Targeted mutation: `diagnostic_ack_exposure` and\n`last_acked_input_frame` mutants caught",
          "timestamp": "2026-11-11T01:51:23-08:00",
          "tree_id": "870137a3f2c0041e07660b3c44bf7cf9df3683f5",
          "url": "https://github.com/wallstop/fortress-rollback/commit/2a360dbd46c8bd4a1d6c2f6ae0d450693cc0c1df"
        },
        "date": 1794391168427,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39292,
            "range": "± 996",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25316,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1243,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2827,
            "range": "± 285",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d6be05cc2ab494c9a80d5f8c6474672a6d884209",
          "message": "test(perf): qualify representative-state tiers and publish budgets (#394)\n\n## Summary\n\nStarts the representative-state gameplay/performance qualification with\na real release workload\nledger. No production behavior changes.\n\n### Harness\n\n`tests/representative_perf.rs` runs real `P2PSession` meshes over an\nin-memory fabric with\nconfigurable snapshot payload, per-frame simulation bytes, player count,\ninput delay, prediction\nwindow, and deterministic message loss. It records:\n\n- frame-service p50/p95/p99/p99.9\n- rollback count, resimulated frames, max depth\n- allocation count and net allocated bytes\n- per-player protocol sent/received bytes\n- minimum final confirmed frame\n\nA 48-frame 2-player smoke row runs in the normal suite. The full\n2/4/16-player ledger is a\nrelease-only ignored test that emits JSONL via `FORTRESS_PERF_OUT` and\nenforces reference p99\nbudgets only with `FORTRESS_PERF_ENFORCE=1`.\n\n### Reference measurements\n\n| Tier | Players | State | Work | p50 | p99 | p99.9 | Rollbacks | Max\ndepth |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n| 2p-rep-64k | 2 | 64 KiB | 1 MiB | 0.035 ms | 0.076 ms | 0.105 ms | 294\n| 2 |\n| 4p-rep-256k | 4 | 256 KiB | 4 MiB | 0.181 ms | 0.499 ms | 0.625 ms |\n729 | 6 |\n| 16p-rep-64k | 16 | 64 KiB | 2 MiB | 0.084 ms | 0.141 ms | 0.149 ms |\n1,839 | 3 |\n\nReference budgets: p99 ≤ 2 ms for 2/4 players and ≤ 4 ms for 16 players\nat the recorded workloads.\nThe row uses instant transport and zero configured input delay to\nisolate snapshot/simulation cost;\nthe existing baseline sweep remains the source for\nmobile/4-player/16-player network-latency cost.\n\n`docs/tuning.md` publishes the command, table, thresholds, and\ninterpretation. The P1 item stays in\nprogress for the playable workload, visible correction/audio\nduplication, queue behavior, and\ntarget-hardware profiling.\n\n## Testing\n\n- `cargo nextest run --features tokio,json`: 3,115 passed, 74 skipped, 0\nfailed\n- Release ledger with `FORTRESS_PERF_ENFORCE=1`: all 3 tiers pass\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`: clean\n- `agent-preflight.py --auto-fix`, markdownlint, typos, version/plan\nhooks: clean\n- Dependencies: `cargo update --workspace --dry-run` selects zero\npackages; only MSRV-held majors\n  remain",
          "timestamp": "2026-11-11T03:31:20-08:00",
          "tree_id": "0e917a07090e4ced70eda0415826fda529285b1a",
          "url": "https://github.com/wallstop/fortress-rollback/commit/d6be05cc2ab494c9a80d5f8c6474672a6d884209"
        },
        "date": 1794397182653,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60595,
            "range": "± 951",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 31933,
            "range": "± 129",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1672,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
            "range": "± 95",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3144,
            "range": "± 179",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e6a0458b43b94c5d2efd51450268a1fec30346ee",
          "message": "test(perf): add playable representative workload and budgets (#396)\n\n## Summary\n\nCompletes the P1 representative-state qualification with a playable\nreference workload and enforced\nbudgets. No production behavior changes.\n\n### Playable workload\n\n`tests/representative_perf.rs` now runs a deterministic 2D reference\ngame (entity movement plus\nsnapshot payload and configured simulation work) through `ChaosSocket`\nlatency, jitter, and loss.\n\n- Visible corrections: rollback loads.\n- Presentation duplications: already-presented frames re-simulated.\n- Adapter queue evidence: sampled `SessionMetrics::socket.queue_depth`\nand `oldest_queued_ms`.\n- Tiers: 2p mobile, 4p wifi, 4p severe, 16p regional.\n\n### Measured release ledger\n\n| Tier | p50 | p95 | p99 | p99.9 | Rollbacks | Visible/dup | Queue\nmax/age |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n| 2p-mobile | 0.038 ms | 0.055 ms | 0.063 ms | 0.186 ms | 347 | 347/374\n| 11/48 ms |\n| 4p-wifi | 0.101 ms | 0.235 ms | 0.514 ms | 0.628 ms | 353 | 353/385 |\n22/32 ms |\n| 4p-severe | 0.164 ms | 0.713 ms | 1.011 ms | 3.303 ms | 346 |\n346/1,398 | 39/96 ms |\n| 16p-regional | 0.086 ms | 0.238 ms | 0.294 ms | 0.309 ms | 981 |\n981/2,687 | 137/48 ms |\n\nBudgets enforced with `FORTRESS_PERF_ENFORCE=1`: p99 ≤2 ms for 2/4\nplayers, ≤6 ms for 16 players,\nand adapter queue age ≤ latency + 2×jitter + 3 frames + 100 ms. All four\ntiers pass.\n\n`docs/tuning.md` publishes the command, table, thresholds, and workload\ncaveats. The P1\nrepresentative-state item is complete and removed from `PLAN.md`.\n\n### Issue closure\n\n- #395 is implemented by this playable workload; close as completed\nafter merge.\n- #390 will be closed as not planned: the 2 s production disconnect\ndefault is intentional, the\nper-PR tier already excludes loss, deterministic progress-under-loss\ncoverage exists, and the\nfailing macOS nightly job passed its rerun. No product change is\nsilently retained.\n\n## Testing\n\n- `cargo nextest run --features tokio,json`: 3,115 passed, 74 skipped, 0\nfailed\n- Release ledger with `FORTRESS_PERF_ENFORCE=1`: all four tiers pass\n- `cargo clippy --workspace --all-targets --features tokio,json,hot-join\n-- -D warnings`: clean\n- `agent-preflight.py --auto-fix`, markdownlint, typos, version/plan\nhooks: clean\n- `cargo update --workspace --dry-run`: 0 compatible updates",
          "timestamp": "2026-11-11T04:51:20-08:00",
          "tree_id": "c0949afe2e8d62b52c442434f486125f3150c6d1",
          "url": "https://github.com/wallstop/fortress-rollback/commit/e6a0458b43b94c5d2efd51450268a1fec30346ee"
        },
        "date": 1794401937644,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39782,
            "range": "± 1876",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24337,
            "range": "± 798",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1246,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2760,
            "range": "± 159",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ffe6c52d5ffd0208b18c41d77ac6ebedb4f7b1ea",
          "message": "fix(examples): propagate request errors instead of warning (#397)\n\n## Summary\n\nCompletes the P1 safe-integration item. Canonical request loops now fail\nearly on missing rollback\nstate and checksum serialization errors instead of warning and\ncontinuing.\n\n### Changes\n\n- New public `try_handle_requests!`: each save/load/advance closure\nreturns a `Result`, and the\nfirst error aborts the request loop. `handle_requests!` remains the\ninfallible convenience form.\n- `examples/request_handling.rs` returns `Result<(),\nRequestHandlingError>` from both manual and\nmacro paths; missing `LoadGameState` state and `ChecksumError`\npropagate.\n- `examples/sync_test.rs` returns a structured internal error for a\nmissing rollback save.\n- `docs/getting-started.md` and `docs/user-guide.md` canonical loops\npropagate errors; macro\n  guidance uses `try_handle_requests!`.\n- New `tests/sessions/misuse.rs` counterexamples: missing saved state,\nchecksum serialization\n  failure, omitted local input (`MissingLocalInput`).\n- New macro regression test proves later requests are not processed\nafter an error.\n- Production checklist maps event starvation, variable-width input,\nconfig mismatch, and NAT\n  rebinding to their deterministic suites.\n- Changelog Added entry; public API census refreshed (2,750 symbols).\n- P1 safe-integration item removed from `PLAN.md`; two future items\nremain.\n\n## Testing\n\n- `cargo nextest run --features tokio,json`: 3,119 passed, 74 skipped, 0\nfailed\n- `cargo clippy --workspace --all-targets --features tokio,json,hot-join\n-- -D warnings`: clean\n- `cargo run --example request_handling`: completes with checksums and\nno warn-and-continue path\n- `agent-preflight.py --auto-fix`, markdownlint, typos,\nchangelog/version/plan hooks: clean\n- `cargo update --workspace --dry-run`: 0 compatible updates",
          "timestamp": "2026-11-11T06:17:46-08:00",
          "tree_id": "47be49114991b630a2c5db0bbf782e19864d3ba8",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ffe6c52d5ffd0208b18c41d77ac6ebedb4f7b1ea"
        },
        "date": 1794407121888,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39547,
            "range": "± 160",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 26773,
            "range": "± 2149",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1246,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2773,
            "range": "± 164",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8294aeadcbfa61356c84a716499de6470f778b66",
          "message": "test(replay): cross-platform replay, mixed transports, and tab suspension (P1) (#400)\n\n## Description\n\nAdvances PLAN P1 (portability and long-session determinism) with three\nevidence classes:\n\n- **Mixed transport replay.** Adds\n`tests/simulation/corpus/002-mixed-transport-quorum.json`, a\nchecked-in 4-peer, 1000-step schedule whose 12 directed links split\nevenly between a reliable\nretransmission transport (every would-drop is converted to a 40ms\nhead-of-line retransmit delay)\nand unreliable UDP-like loss (2% drop, 1% duplication, 25ms base + 10ms\njitter). A hold/release\nand a temporary block cross both classes before the final heal. The\ncorpus replay and strict\nsafe-release lanes run the full oracle and a second replay asserts an\nidentical trace hash and\n  final confirmed frames.\n- **One captured replay on every target.** Adds the\n`fortress-replay-fixture` test-support crate\nwith a 6171-byte replay captured from a real two-peer session with\nrollback corrections. Native,\nbrowser `wasm32-unknown-unknown`, and Godot `wasm32-unknown-emscripten`\nlanes load the same bytes,\nreplay with checksum validation, and assert frames=149, requests=298,\nzero `ReplayDesync`\nevents, and request/state digest `0xb3535a3db29da7784abaacf609fec42b`. A\ncorrupted-checksum\n  negative control proves the validation path has teeth.\n- **Background-tab suspension.** The browser runtime test now injects a\nvirtual clock. A 1s\nsuspension (above the 500ms interruption notice, below the 2s timeout)\nmust emit\n`NetworkInterrupted` then `NetworkResumed`, must not disconnect, and\nmust keep peer states\nidentical after resuming. A suspension past 2s must emit `Disconnected`\nfail-closed.\n\nPLAN now records extended frame/ring operation as the remaining P1 item.\nThe production checklist\nand its wiki mirror document the new cross-platform and mixed-transport\nevidence.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix\n- [ ] ✨ New feature\n- [ ] 💥 Breaking change\n- [x] 📚 Documentation\n- [ ] ♻️ Refactor\n- [x] 🧪 Test\n- [x] 🔧 CI/Build\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] Zero-panic policy: no new production code, and the test-support\ncrate returns `Result` for all\n      fallible replay operations.\n- [x] Added tests that prove the evidence is real (mixed-class counters,\nfull oracle, replay digest,\ncorrupted-checksum negative control, suspension resume and timeout).\n- [x] `cargo fmt` and `cargo clippy --workspace --all-targets --features\ntokio,json -- -D warnings`\n      pass.\n- [x] `cargo nextest run --features tokio,json` passes.\n\n### If Applicable\n\n- [x] Documentation updated (production checklist + wiki mirror + PLAN).\n- [ ] `CHANGELOG.md`: no public API or user-observable behavior changed;\ntest/CI/docs only.\n- [ ] Examples: not affected.\n- [x] No new compiler warnings.\n\n## Testing\n\n**Tests added/modified:**\n\n-\n`simulation::corpus_replay::checked_in_mixed_transport_schedule_replays_with_both_transport_classes`\n-\n`fortress-replay-fixture::tests::checked_in_fixture_replays_with_expected_evidence`\n-\n`fortress-replay-fixture::tests::mutated_recorded_checksum_is_detected`\n-\n`wasm-browser-smoke::captured_native_replay_replays_identically_in_browser`\n-\n`wasm-browser-smoke::browser_tab_suspension_within_timeout_resumes_and_advances`\n-\n`wasm-browser-smoke::browser_tab_suspension_past_timeout_disconnects_fail_closed`\n- Godot Emscripten probe + Playwright assertions for the same replay\nevidence.\n\n**Local evidence:**\n\n```\ncargo nextest run --features tokio,json\n  Summary [167.933s] 3120 tests run: 3120 passed, 74 skipped\n\ncargo nextest run -p fortress-replay-fixture\n  Summary 2 tests run: 2 passed\n\ncargo nextest run --test simulation -E 'test(simulation::corpus_replay)'\n  Summary 10 tests run: 10 passed\n  (mixed schedule: retransmit_delayed=6940, dropped_by_policy=137,\n   final confirmed=[813, 813, 813, 813])\n\ncargo clippy --workspace --all-targets --features tokio,json -- -D warnings\ncargo clippy -p wasm-browser-smoke --target wasm32-unknown-unknown --all-targets -- -D warnings\ncargo clippy --manifest-path tests/godot-emscripten/Cargo.toml --locked --all-targets --all-features -- -D warnings\ncargo shear, actionlint, agent-preflight, markdownlint, typos, doc links, doc claims: clean\n```\n\nBrowser and Emscripten execution evidence is produced by the hosted\n`WASM Build Check` and\n`Godot Emscripten Browser` jobs on this PR; the local container has no\nChrome or emsdk. The\nsuspension/resume/timeout semantics were pre-validated natively through\nthe same virtual-clock\nsequence (interrupted+resumed under 1s, interrupted+disconnected past\n2s).\n\n## Related Issues\n\nRelates to PLAN P1 (portability and long-session determinism). No open\nissues at session start.",
          "timestamp": "2026-11-11T19:24:48-08:00",
          "tree_id": "6132f26d0516163200c55089d42821d9ae90cdf0",
          "url": "https://github.com/wallstop/fortress-rollback/commit/8294aeadcbfa61356c84a716499de6470f778b66"
        },
        "date": 1794454349450,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39581,
            "range": "± 491",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25635,
            "range": "± 458",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1246,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2766,
            "range": "± 159",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0b76a47782d8a44e88ccab298536c60e85519438",
          "message": "test(replay): extended frame/ring evidence and nightly host-stall tolerance (#401)\n\n## Description\n\nCompletes the final PLAN P1 item — extended frame/ring operation — and\nfixes the recurring macOS\nnightly real-UDP host-stall failure.\n\n- **Extended portable replay.** The checked-in two-peer capture grows\nfrom 149 to 4,096 replayed\nframes (68.3 s at 60 Hz). It recycles the 128-frame input queue 32 times\nand the default\nnine-cell saved-state ring more than 450 times. Playback now records an\nordered request-stream\ndigest and re-encodes the decoded replay to assert the checked-in\nartifact is canonical, so\nnative, browser `wasm32-unknown-unknown`, and Godot\n`wasm32-unknown-emscripten` lanes prove both\nthe request sequence and the serialized artifact are byte-identical.\nNon-vacuity tests assert\nthe ring-cycle counts and that every replayed frame carries a recorded\nchecksum; the existing\n  corrupted-checksum control still detects a mutated checksum.\n- **Extended live corpus schedule.** New\n`tests/simulation/corpus/003-extended-ring-recycling.json` runs 4 peers\nfor 16,384 steps at the\nminimum prediction window over mixed reliable-retransmission and\nunreliable links. Sixteen\nhold/block/stall/delay-increase cycles keep rollback and ring recycling\nactive, and the\ndedicated replay test asserts the full oracle, final confirmed prefixes\npast 12,000 frames, and\n  identical trace identity on a second run.\n- **Nightly host-stall RCA (closes #390).** The scheduled nightly run on\n`4a84892` failed only\nmacOS `test_extended_chaos_bursty_survivable` after 180 s with both\npeers in `Synchronizing`\n(`current=8 confirmed=NULL_FRAME`, `synced=1 interrupted=1\ndisconnected=1`) — the same signature\nas #390. A hosted runner can lose the CPU for longer than the production\ntwo-second disconnect\ntimeout, so loss-bearing nightly real-UDP peers now opt into a test-only\n10 s session timeout\nvia `--disconnect-timeout-ms`. Loss-free nightly rows (including the\ndeliberate 2 s\nstaggered-startup disconnect/retry test) and the per-PR smoke tier keep\nthe production default,\nand production-timeout semantics remain covered by deterministic\nin-process suites.\n- `PLAN.md` drops the completed P1 item, leaving only the future P2\nsearch experiment.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change\n- [x] 📚 Documentation\n- [ ] ♻️ Refactor\n- [x] 🧪 Test\n- [x] 🔧 CI/Build\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] Zero-panic policy: no production code added; fixture helpers\nreturn `Result`, and test-only\n      peer parsing uses the existing harness conventions.\n- [x] Added tests that prove the evidence is real (ring-cycle counts,\nrequest/state digests,\ncanonical artifact bytes, full oracle replay identity,\ncorrupted-checksum control).\n- [x] `cargo fmt` and `cargo clippy --workspace --all-targets --features\ntokio,json -- -D warnings`\n      pass.\n- [x] `cargo nextest run --features tokio,json` passes (3,121 passed, 74\nskipped).\n\n### If Applicable\n\n- [x] Documentation updated (production checklist + wiki mirror + PLAN).\n- [ ] `CHANGELOG.md`: no public API or user-observable behavior changed;\ntest/CI/docs only.\n- [ ] Examples: not affected.\n- [x] No new compiler warnings.\n\n## Testing\n\n**Tests added/modified:**\n\n- `fortress-replay-fixture`: extended capture/playback evidence,\nrequest-stream digest,\ncanonical-artifact check, ring-coverage and checksum-coverage\nassertions.\n- Browser smoke and Godot probes assert the new request digest and\n4,096-frame evidence.\n-\n`simulation::corpus_replay::checked_in_extended_ring_schedule_recycles_rings_and_replays_identically`.\n- Nightly peer binary `--disconnect-timeout-ms` parsing and session\napplication.\n\n**Manual testing performed:**\n\n- `cargo nextest run --features tokio,json --no-capture` — 3,121 passed,\n74 skipped.\n- `cargo nextest run --features hot-join,tokio,json --no-capture` —\n3,385 passed, 75 skipped.\n- Browser lane locally against Chrome for Testing 157.0.8103.78 — 5/5\npassed, including the\n  extended replay and both tab-suspension tests.\n- Safe-release corpus replay with `-C overflow-checks=on -C\ndebug-assertions=on` — 12 passed.\n- Full 47-test nightly real-UDP suite in release with the host-stall\ntolerance applied — passed.\n- Capture determinism: two captures are byte-identical; artifact is\n167,998 bytes and replays with\n  zero desyncs.\n- Agent preflight, links, doc claims, wiki consistency, plan hygiene,\ntypos, markdownlint,\n  actionlint, shell portability, and network timing invariants pass.\n\n## Related Issues\n\nCloses #390.",
          "timestamp": "2026-11-11T22:17:01-08:00",
          "tree_id": "9c6bf70a9edc272bd3c4c29a56cdb22184ef07ff",
          "url": "https://github.com/wallstop/fortress-rollback/commit/0b76a47782d8a44e88ccab298536c60e85519438"
        },
        "date": 1794464690497,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39660,
            "range": "± 852",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 26010,
            "range": "± 190",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1244,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2853,
            "range": "± 301",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "caa243eb42e7ace805f284fec2e9ef6e5b7fbec7",
          "message": "test(simulation): make rare-state census non-vacuous (P2) (#404)\n\n## Description\n\nThe nightly sometimes-state census was stable but non-vacuous for only\n2/5 IDs. Ten consecutive hosted windows (2026-11-03 through 2026-11-12)\nreported the floor and nudge rows at stable rates (620-630 and 893-902\nper 1,000 schedules) while prediction-limit, sparse-earlier, and\ninput-ring stayed at zero. The default nightly session uses a 128-slot\nring, `max_prediction = 8`, `input_delay = 1`, and\n`SaveMode::EveryFrame`, so two of those rows were structurally\nunreachable. Issue #316 requires at least 3/5 eligible IDs, so the\noriginal organic presence ratchet is rejected.\n\nThis change makes the census non-vacuous without changing the fault mix\nfor the existing 96% of rows:\n\n- Add an optional, serde-defaulted `SimConfig::input_queue_length` axis\nand wire it into the normal and hot-join replacement builders.\n- Reserve 40/1,000 nightly rows for a two-slot ring with `max_prediction\n= 1` and `input_delay = 0`, using `seed % 100` residues that are\nhistorically `Clean`, never `Swarm`, and span 2- through 16-player\nmeshes.\n- Validate the effective ring (including the 128-slot default) against\n`input_delay + max_prediction`, reject every `SetInputDelay` that would\nexceed it, and require schema 20 for an explicit override.\n- Fix the shrink regression fixture that combined a 127-frame delay with\nan eight-frame prediction window, which session construction rightly\nrejects.\n- Update `PLAN.md` to the next P2 step: collect seven post-redesign\nwindows and add the presence ratchet if at least 3/5 IDs remain stable.\n\nA local 1,000-schedule window reports `prediction=40`, `floor=617`,\n`nudge=897`, `sparse-earlier=0`, `input-ring=40` over 5,000,000 steps\nand 9,025 peer sessions (4/5 eligible). The remaining sparse-earlier gap\nis tracked in #402; the ratchet scope is #403.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n-\n`nightly_rare_state_coverage_cell_is_present_and_exercises_ring_and_prediction_states`\nchecks the 40-row share, per-shard presence, 2- and 16-player edges,\nschedule shape, full-oracle pass, both state hits, and exact replay\nidentity.\n- Schedule validation tests cover ring bounds, retained-window overflow\non explicit and default rings, `SetInputDelay` overflow, schema floor,\nand legacy serialization omission.\n- Full nightly window locally: 8/8 shards, 1,000 schedules / 5,000,000\nsteps; targeted probes 4/4.\n\n**Manual testing performed:**\n\n- `cargo nextest run --features tokio,json --no-capture` — 3,124 passed,\n74 skipped.\n- `cargo nextest run --features hot-join,tokio,json --no-capture` —\n3,388 passed, 75 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings` — clean.\n- `cargo test --release --test simulation --features hot-join\n'simulation::fleet::'` — 76 passed.\n- `cargo test --release --test simulation --features hot-join\n'simulation::harness::schedule::tests::'` — 39 passed.\n- `python3 scripts/ci/agent-preflight.py --auto-fix`, plan hygiene,\nmarkdownlint, typos, link checks, and version sync — clean.\n\n## Related Issues\n\nRelates to #316 (original census contract), #402 (sparse-earlier organic\ngap), and #403 (seven-window ratchet scope).",
          "timestamp": "2026-11-12T09:05:00-08:00",
          "tree_id": "ba92c7150ff8cde3273ac2c8fde829241e648ba6",
          "url": "https://github.com/wallstop/fortress-rollback/commit/caa243eb42e7ace805f284fec2e9ef6e5b7fbec7"
        },
        "date": 1794503561493,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39506,
            "range": "± 688",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25505,
            "range": "± 726",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1245,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2952,
            "range": "± 406",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ec9ac03fbe84acca7ea79bd727302ce4ef3a3c7a",
          "message": "test(simulation): add seven-window organic presence ratchet (P2) (#405)\n\nRecord the sparse-earlier sometimes-state as targeted-only and add the checked-in seven-window evaluator required by #403. Validated against seven scheduled-main artifacts: pre-redesign baseline rejects at 2/3 required eligible IDs.",
          "timestamp": "2026-11-12T10:27:50-08:00",
          "tree_id": "ef3e9f081e05b8b2bfc628c35278597e9dc6ed6c",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ec9ac03fbe84acca7ea79bd727302ce4ef3a3c7a"
        },
        "date": 1794508628185,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39297,
            "range": "± 1961",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24613,
            "range": "± 580",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1246,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2789,
            "range": "± 226",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "83013d983262e36ee568c9bf49efa8bf265ad183",
          "message": "test: satisfy strict clippy 1.100 lints (#406)\n\nThe stable toolchain moved to 1.100.0 between the PR and main runs; fix every new redundant-clone and needless-range-loop hit in test code so the strict clippy gates stay green.",
          "timestamp": "2026-11-12T11:22:08-08:00",
          "tree_id": "b95d0e166195c42b230baa4150f1a3392532b102",
          "url": "https://github.com/wallstop/fortress-rollback/commit/83013d983262e36ee568c9bf49efa8bf265ad183"
        },
        "date": 1794511869262,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 63587,
            "range": "± 1663",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 31148,
            "range": "± 370",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1651,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3131,
            "range": "± 178",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "fe0b935910de6d34abb062fd97ac3d9eb2aea440",
          "message": "test(simulation): publish the violation-site exercise census (P2) (#412)\n\nJoin the checked-in report_violation! baseline with main coverage and exact nightly fleet observations, publish the unexercised set from a nightly job, and enable tokio,json,hot-join in coverage (recovering 12 hot-join sites; 13 -> 25 exercised). Also replace the hardcoded action-pin allowlist with immutable-SHA shape checks and fold in the dependabot action bumps.",
          "timestamp": "2026-11-12T22:45:22-08:00",
          "tree_id": "21e4c0661222ed57744ad3854c7b26384082830e",
          "url": "https://github.com/wallstop/fortress-rollback/commit/fe0b935910de6d34abb062fd97ac3d9eb2aea440"
        },
        "date": 1794552779317,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39543,
            "range": "± 574",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25713,
            "range": "± 182",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1243,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2810,
            "range": "± 221",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "3db1458d59ae7fe18431e1192f16820bc10ec94a",
          "message": "test(simulation): make violation-site exercise census exact (#413)\n\n## Summary\n\nCloses #411.\n\nThe nightly violation-site exercise census joined the content-addressed\nproduction\n`report_violation!` baseline with Cobertura line coverage and fleet\nshards. Coverage is only a\nconservative lower bound for macro invocations: LLVM can attribute an\nexpansion to surrounding\nlines, so genuinely executed assertions stayed in the unexercised set.\nThis change adds an exact\nruntime source and folds it into the nightly report.\n\n## Changes\n\n- **Opt-in `violation-census` feature.** `SpecViolation::new` calls a\ncold-path hook that records\neach new `file:line` location in a `parking_lot`-guarded `BTreeSet` (256\nlocations per process).\nWhen `FORTRESS_VIOLATION_CENSUS_DIR` is set, each new location rewrites\na bounded\n`observations-<pid>.json` snapshot with sorted unique entries, an\noptional workflow `run_id`, and\na `truncated` marker when the 256-location or 32 KiB limit drops\nentries. The hook compiles out\nof normal builds and Loom builds, never panics, and treats every\nfilesystem error as diagnostic.\n- **Exact observations job.** The nightly workflow runs the full suite\nwith\n`--features tokio,json,hot-join,violation-census` and uploads the\nbounded artifacts.\n- **Census join.** `violation_census.py` gained `--observations-dir`,\n`--expected-run-id`, and\n`--partial` with artifact-count, per-artifact size, per-artifact\nlocation, and processed-location\ncaps plus schema, run-id, process-id, filename, and order validation.\nAbsolute and Windows paths\nnormalize without promoting test-local `src/` labels. Test-only labels\ncount as unmatched;\nmalformed `src/` locations still fail loudly; unmatched source and\nignored entries are surfaced\n  in the report. Empty fleet or observation inputs are rejected.\n- **Failure behavior.** The census runs with `if: always()`, tolerant\nartifact downloads, and marks\nthe report `partial` when a producer job or either artifact download\nfails.\n- Documented the feature in `Cargo.toml`, the user guide, migration\nguide, wiki mirrors, and\nchangelog, and added it to the cargo-geiger and strict-clippy feature\ncensuses.\n- Fixed a pre-existing non-line `SpecViolation` location on the\nscoped-observer drop path so\n  diagnostic consumers and the census can parse it.\n- Updated the P2 plan item to the exact-source next step.\n\n## Evidence\n\n- Full suite with the hook: **3,399 passed, 76 skipped**; 297 artifacts,\n163 observed locations,\n  max 5 per artifact, 0 over 64 KiB, one run id.\n- Census against those observations: **109/268 exercised** (vs. 25/268\nfrom coverage alone),\n109 matched, 54 unmatched (41 under `src/` surfaced for review), 0\ntruncated.\n  `src/network/compression.rs` no longer appears in the unexercised set.\n- Downloaded the latest main coverage artifact (`fe0b935`, run\n43380911481): coverage-derived IDs\n`0`, exact-observation IDs `109`; the exact source is currently a strict\nsuperset.\n- `cargo clippy --workspace --all-targets --features\ntokio,json,violation-census -- -D warnings`:\nclean. `cargo check --no-default-features --features violation-census`:\nclean. Both census tests\n  pass, including snapshot replacement and byte-budget truncation.\n- `python3 -m pytest scripts/tests/ -q`: **2,227 passed**; `mypy`:\nclean.\n- `ruff check`/`ruff format --check`, `actionlint`,\n`check-plan-hygiene`,\n`check-changelog-unreleased`, `sync-version --check`, `check-links`,\n`check-doc-claims`,\n  `check-unbounded-alloc`, and `agent-preflight.py --auto-fix`: clean.\n\nNo public API change; the hook is feature-gated and absent from normal\nbuilds.",
          "timestamp": "2026-11-13T19:59:28-08:00",
          "tree_id": "5a95a3c37f2b3175435b146422036faa597e0b1b",
          "url": "https://github.com/wallstop/fortress-rollback/commit/3db1458d59ae7fe18431e1192f16820bc10ec94a"
        },
        "date": 1794629215530,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39491,
            "range": "± 393",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25607,
            "range": "± 154",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1245,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2789,
            "range": "± 170",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "482ec27e9e6473f31d3bd08451181b2a5b79258f",
          "message": "test(simulation): complete and cut the exact violation-site census (#414)\n\nInventory report_violation_to! sites, wrapper-macro call sites, and placeholder-first messages; regenerate the 284-site baseline; delete two redundant receive-path asserts; cover 36 previously unexercised sites with focused tests. Local exact census: 154/284 exercised (130 unexercised), down from 159 unexercised in nightly run 43555370869.",
          "timestamp": "2026-11-13T21:23:36-08:00",
          "tree_id": "962e69a2812ca55ab94cdfcfc4361ee3dab7a3bd",
          "url": "https://github.com/wallstop/fortress-rollback/commit/482ec27e9e6473f31d3bd08451181b2a5b79258f"
        },
        "date": 1794634303012,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60534,
            "range": "± 972",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 31110,
            "range": "± 198",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1862,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3139,
            "range": "± 176",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e64f23ad8d258f0c006d3789d7dcde04fba3d61a",
          "message": "test: cover reachable violation-site guards (P2) (#415)\n\nExercise the reachable defensive guards behind the remaining unexercised violation-site inventory. Local exact census moves 154/284 -> 202/284 exercised (130 -> 82 unexercised). Test-only change; no production behavior.",
          "timestamp": "2026-11-13T22:56:20-08:00",
          "tree_id": "e6c74af16e17544ca19a2200197803cc67e25420",
          "url": "https://github.com/wallstop/fortress-rollback/commit/e64f23ad8d258f0c006d3789d7dcde04fba3d61a"
        },
        "date": 1794639875811,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 43906,
            "range": "± 187",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25497,
            "range": "± 213",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 674,
            "range": "± 76",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 869,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 6940,
            "range": "± 274",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b9ce93840086253f65a0123a8ac1bbea66899ed2",
          "message": "test(simulation): cut the residual violation-site census (P2) (#417)\n\n## Description\n\nExercise the remaining reachable defensive `report_violation!` guards\nbehind the exact violation-site census, completing stage 2 of the\nfleet-search violation-coverage item. Local exact observations move from\n**202/284 to 243/284 exercised** (82 -> 41 unexercised). Test-only\nchange; no production behavior changed.\n\nThe 41 residual sites are classified in #416:\n\n- 19 allocation/encode/writer fallbacks with no fault-injection hook.\n- 14 unreachable-by-construction postconditions / duplicate guards\n(deletion needs maintainer-approved baseline regeneration).\n- 8 reachable defensive fallbacks needing targeted fault injection\n(snapshot capture serialization, replay recorder failure, endpoint\nre-arm, propagated/timed-out disconnect application).\n\n`PLAN.md` now hands the residual classification to #416 and keeps the\ncalendar-bound seven-window ratchet item.\n\n## Type of Change\n\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly (`PLAN.md` only; no\nuser-facing docs or changelog entry for test-only work)\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n(not applicable: test-only)\n- [ ] I have updated relevant examples in the `examples/` directory (not\napplicable)\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `src/sync_layer/mod.rs`: unaddressable `save_current_state` frame\nfallback; reactivated-slot `synchronized_inputs` pre-activation bound.\n- `src/input_queue/mod.rs`: gap-fill missing replication source;\n`reset_to_frame` delayed-frame underflow.\n- `src/sessions/p2p_spectator_session.rs`: missing host connect-status\nrow; canonical commit missing input slot/status.\n- `src/sessions/p2p_session.rs`: missing synchronized inputs;\nout-of-window hot-join activation; staged 2-peer and N-peer serve\nfailure paths; N-peer abort-before-capture; joiner backfill guards;\nremote disconnect snapshot fallbacks; staged survivor pending\nreactivation; constructor guards; queued coordinated-drop guard;\ntruncated spectator batch; 2-peer joiner apply failure; N-peer buffered\njoiner coordinator loss/apply failure.\n- `src/network/protocol/mod.rs`: armed failing `Config::Input`\nserializer for the `enqueue_replicated_input` serialization fallback.\n\n**Manual testing performed:**\n\n- `cargo +1.100.0 fmt --check` (CI-matching toolchain): clean.\n- `cargo +1.100.0 clippy --workspace --all-targets --features\ntokio,json,hot-join,violation-census -- -D warnings`: clean.\n- `cargo +1.100.0 clippy --workspace --all-targets -- -D warnings`:\nclean.\n- `cargo +1.100.0 nextest run --profile ci --features\ntokio,json,hot-join,violation-census`: 3467 passed, 76 skipped.\n- `cargo +1.100.0 nextest run`: 3141 passed, 75 skipped.\n- Exact census: 284 baseline, 243 exercised, 41 unexercised.\n- `python3 scripts/ci/agent-preflight.py --auto-fix`: all checks passed.\n\n## Related Issues\n\n- Relates to #416\n- Relates to #403",
          "timestamp": "2026-11-14T10:37:01-08:00",
          "tree_id": "8f629670cbf2ea972ec98d6ecf806dfbb3b5f699",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b9ce93840086253f65a0123a8ac1bbea66899ed2"
        },
        "date": 1794681920271,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60563,
            "range": "± 1946",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 31944,
            "range": "± 396",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1862,
            "range": "± 111",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2866,
            "range": "± 282",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "90057ce477d8fdbfe33239099e29a425becd32df",
          "message": "test(simulation): classify residual violation sites and harden the ephemeral port test (#419)\n\n## Description\n\nTwo focused test-integrity changes plus the residual violation-site\nclassification:\n\n1. **#418 — flaky parallel ephemeral binding test.** The test dropped\neach worker's sockets after a fixed `thread::sleep(10ms)` instead of\nholding them until every thread finished binding. A socket bound early\ncould return its ephemeral port to the OS while other threads were still\ncalling `bind(0)`, so a later bind could receive a recycled port that\nthe shared address set still contained and report a false duplicate.\nWorkers now return their still-bound sockets through `join()`, the main\nthread retains all 40 until the uniqueness assertion, and the racy sleep\nis gone.\n\n2. **#416 — residual violation-site classification.** The exact local\ncensus moves from **243/284 to 244/284** exercised:\n- `p2p_session.rs:3704` (\"Failed to capture hot-join snapshot\") is now\nexercised exactly by\n`hot_join_oversized_snapshot_capture_failure_keeps_host_running`: a host\nwhose `hot_join_max_snapshot_wire_bytes` cap rejects every capture\nreports the violation, sends no snapshot, and keeps advancing solo (the\ntest asserts ≥100 frames of progress across 120 polls, so a paused serve\ncannot pass).\n- **13 unreachable-by-construction** checks carry a short `Unreachable\nby construction:` proof and remain as fail-closed defense.\n- **19 allocation/encode fallbacks** and **8 sequence-dependent\ndefensive arms** are recorded as intentionally unexercised without an\nallocator/writer fault hook or adversarial sequencing; the policy is\ndocumented in `scripts/simulation/violation_census.py`.\n- The sparse save-mode `last_saved_frame` mismatch was reclassified as a\n**reachable residual**: when a gossip-lowered confirmed frame sits below\nthe resolved (clamped) rollback target, the re-simulation loop never\nreaches that frame. It is no longer claimed unreachable.\n\n`PLAN.md` drops the completed #416 item; the calendar-bound #403 item\nstays.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `hot_join_oversized_snapshot_capture_failure_keeps_host_running`\n(new): exact lenient-path coverage for the rejected snapshot capture;\nasserts the specific violation kind/severity, zero snapshots on the\nwire, the joiner still `HotJoining` at the first rejection, and ≥100\nhost frames of progress.\n- `parallel_ephemeral_binding_no_conflicts`: keeps every socket bound\nthrough all threads' binds and asserts the retained count.\n\n**Manual testing performed:**\n\n- Exact local census (`--profile ci --features\ntokio,json,hot-join,violation-census`): **3468 passed, 0 failed (1\nslow)**, then `violation_census.py` reports `284 baseline, 244\nexercised, 40 unexercised`, and the observation artifact contains\n`src/sessions/p2p_session.rs:3704`.\n- 50 consecutive `ephemeral_port_tests` runs: 50 × 25 passed, zero\nfailures.\n- `cargo clippy --workspace --all-targets --features\ntokio,json,hot-join`: clean.\n- `python3 scripts/ci/agent-preflight.py --auto-fix`: all checks passed.\n- `python3 scripts/simulation/violation_sites.py --check`: 284 entries\nmatch.\n- Latest `main` CI at `b9ce938`: all workflows success. `cargo\noutdated`: only MSRV-held versions Dependabot is configured to ignore.\n\n## Related Issues\n\nFixes #418\nCloses #416\nRelates to #403",
          "timestamp": "2026-11-14T12:24:39-08:00",
          "tree_id": "35c6d7c66fb0bb3864cef8d559bf1022a3e81402",
          "url": "https://github.com/wallstop/fortress-rollback/commit/90057ce477d8fdbfe33239099e29a425becd32df"
        },
        "date": 1794688337436,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 40124,
            "range": "± 525",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25256,
            "range": "± 991",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1090,
            "range": "± 38",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1244,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2830,
            "range": "± 315",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e6ad6c17ccfbdfdbd8a726444920a203ee7a857b",
          "message": "test(simulation): bound PCT interleaving and fix a lagging-survivor freeze false positive (#421)\n\n## Summary\n\n- Adds an opt-in, bounded PCT peer-interleaving axis to the\ndeterministic simulation harness (`RunOptions::pct`, at most 1,024\npriority assignments). The historical forward order is unchanged when\nunset, and `pct` is mutually exclusive with differential mode.\n- Uses the axis to run a matched equal-compute comparison against the\ncurrent forward-order scheduler on the nightly schedule generator.\n- Fixes a freeze-frame oracle false positive exposed by the comparison:\na survivor whose confirmed prefix ends *before* another survivor's\nobserved freeze frame was treated as an authoritative `None`\ndisagreement.\n- Records the PCT rejection in the fleet-search roadmap and narrows\n`PLAN.md` to the remaining prefix-replay and LDFI-lite comparison\n(scoped by #420).\n\n## Measured comparison\n\nMatched arms: run id `202611140101`, shard 0, 128 schedules, 640,000\nscheduled steps each.\n\n| Arm | unique organic vectors | unique peer-count buckets | unique\nbehavior signatures | failures |\n| --- | ---: | ---: | ---: | ---: |\n| Forward (baseline) | 6 | 13 | 107 | 0 |\n| PCT, K=16 | 6 | 13 | 107 | 0 |\n\nBoth arms observed exactly `{0, 2, 4, 6, 17, 21}` (5-bit OR of the\nregistry; sparse-earlier remains unreachable organically). Every trace\nhash changed, proving the interleaving was applied. No coverage or\ndefect-discovery gain, so PCT stays out of the nightly fleet; the opt-in\naxis is retained for order-sensitivity regressions.\n\n## Bug found and fixed\n\nA 1-change PCT run on the checked-in D14 lossy graceful-remove schedule\nproduced:\n\n```text\nFreezeFrameDivergence { slot: 4, peer: 3, first_author: 0, expected: None,\n  actual: Some(FreezePoint { frame: 302, ... }) }\n```\n\nThe step-302 visit order left peer 0 at confirmed frame 301 while peer 3\nreached 303 and observed the slot-4 freeze at 302. The oracle treated\npeer 0's not-yet-observed `None` as a disagreement. A survivor that has\nnot reached the freeze frame cannot disagree; a survivor that did reach\nit and never observed a stable `Disconnected` run still fails. Pinned\nby:\n\n- `oracle_ignores_lagging_survivor_before_the_freeze_frame` (unit).\n- `pct_lagging_survivor_does_not_fake_a_freeze_divergence` (D14\nintegration + exact replay).\n- `oracle_ignores_all_survivors_lagging_behind_the_removal_cut`\n(all-`None` case).\n\nAdversarial review follow-up also clarified that PCT shrinking preserves\nthe stable failure class rather than the exact original trace, and\nrefreshed the stale artifact-schema message after the v9 bump.\n\n## Verification\n\n- `cargo nextest run --profile ci --test simulation --features hot-join`\n→ 473 passed, 27 skipped.\n- `cargo nextest run --profile ci --features tokio,json` → 3,184 passed,\n75 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings` → clean.\n- `cargo clippy --tests --test simulation --features hot-join -- -D\nwarnings` → clean.\n- `python3 scripts/ci/agent-preflight.py --auto-fix` → all checks\npassed.\n- Plan hygiene, Agent Skill validation/quality, markdownlint, link\ncheck, typos → green.\n\n## Issue notes\n\n- #403 remains calendar-bound (2/7 scheduled post-redesign windows; five\nremain through 2026-11-19).\n- #420 records the entrance gate and acceptance criteria for\nprefix-replay branching and LDFI-lite.\n- No production behavior, wire format, replay format, or public API\nchanges. No changelog entry is required for test-only work.",
          "timestamp": "2026-11-14T15:26:14-08:00",
          "tree_id": "321a75ff934b20387dba9e66bddac1c28811f741",
          "url": "https://github.com/wallstop/fortress-rollback/commit/e6ad6c17ccfbdfdbd8a726444920a203ee7a857b"
        },
        "date": 1794699255636,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 61319,
            "range": "± 978",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 31905,
            "range": "± 577",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1693,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 98",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3167,
            "range": "± 198",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "fc6a5f55eef3967f191a17bfea9594a578896c6d",
          "message": "docs(dst): record measured fleet-search rejections (#420) (#424)\n\n## Summary\n\nCompletes the remaining fleet-search comparison in #420 by recording\nmeasured rejections for prefix-replay branching and LDFI-lite, and\nrefreshes lockfile patch updates across every workspace.\n\nPLAN.md now carries only the calendar-bound seven-window organic\npresence ratchet (#403).\n\nCloses #420.\n\n## Measured comparison\n\n### Prefix-replay branching: rejected\n\n- Four 250-step rare-state prefixes (`input_queue_length = 2`,\n`max_prediction = 1`, clean network, `ContinueWithout`) x 64 matched\nsuffixes = 256 branches / 256,000 simulated steps.\n- Every branch reproduced its prefix cumulative trace hash exactly at\nthe cut.\n- Branch arm: only the `0x15` prefix vector (prediction-limit, nudge,\ninput-ring), zero new organic vectors, 3 per-ID peer-count buckets, 0\nfailures.\n- Matched fresh-seed arm: `0x15` and `0x17` (adds floor-round), floor\nrow in 86/256 schedules, 7 buckets, 0 failures.\n- Replaying a prefix consumes suffix reach without a session-state\nsnapshot API; no state discovery per compute.\n\n### LDFI-lite: rejected\n\n- 16 matched 1,000-step green schedules with a bounded 4,096-delivery\nlog and greedy support removal.\n- Greedy support sets of 1-4 deliveries in 15/16 schedules; the\nremaining schedule still confirmed after every candidate was removed.\n- Support removal lowered the target's final confirmed frame by one\nframe in 15/15 tested schedules, with 0 oracle failures.\n- 240 matched random drop sets of the same size lowered it in 0/240,\nwith 0 oracle failures; baseline had 0 failures.\n- LDFI-lite is a stronger targeted confirmation perturbation (15/15 vs\n0/240) but observed no oracle failure in the matched green arms, so the\ndelivery ring, drop-set plumbing, and greedy re-run search are not\nretained.\n\n### Detector control\n\nThe state-discovery arm does not consume oracle failures. The shared\noracle's failure path is pinned by the existing `corrupt_state_from`\nnegative control\n(`artifact_reproduction_rejects_mutated_progress_evidence`, which\nasserts a corrupted run fails). The zero-failure LDFI result is \"no\noracle failure observed in the matched green arms,\" not a claim that the\nfleet is defect-free.\n\nThe experiment instrumentation was removed after recording the result;\nthe reusable measurements and method live in the fleet-search roadmap.\n\n## Verification\n\n- `cargo tf`: 2,738 passed.\n- `cargo tfull`: 3,195 passed, 75 skipped.\n- `cargo fmt --all --check`: passed.\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix`: all checks\npassed.\n- `python3 scripts/hooks/check-plan-hygiene.py`: passed.\n- `python3 scripts/hooks/validate-agent-skills.py`: 49 skills valid.\n- `npx markdownlint` and `typos` on changed docs: passed.\n- `python3 scripts/release/workspace_locks.py check`: passed.",
          "timestamp": "2026-11-15T04:15:33Z",
          "tree_id": "e15399845227ead8a8f7e0e837e8f82512bf85d6",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/fc6a5f55eef3967f191a17bfea9594a578896c6d"
        },
        "date": 1794716868094,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 27243,
            "range": "± 549",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 16724,
            "range": "± 334",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 493,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 615,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 5667,
            "range": "± 285",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8ecd550794978d4be9e3cb580faac0e3a47f114a",
          "message": "test(simulation): enable the organic sometimes-state presence ratchet (#403) (#425)\n\nCloses #403.\n\n## Summary\n\nEnable the organic sometimes-state presence ratchet from seven\npost-redesign nightly windows\n(issue #403), completing the final stage-1 measurement step of the\nfleet-search roadmap.\n\n- Adds `RATCHET_RETAINED_IDS` with the four IDs that recorded organic\n`schedules_hit >= 1` in\n  all seven windows.\n- Adds `OrganicSummary::has_presence` as the single presence predicate\nshared by the seven-window\n  evaluator and the new per-window gate.\n- Extracts `enforce_retained_presence`, called by the nightly census\nmerge: a retained ID with\nzero organic schedules now fails `CI - Simulation Tests (Nightly)`. The\nworkflow step is named\n  to show it gates.\n- The ignored seven-window evaluator now fails when the evidence is\nrejected and asserts its\nretained set equals the checked-in constant, so the evidence and the\ngate cannot drift.\n- Tests cover registry-wide gate membership, peer-session hits not\nsubstituting for schedule\n  presence, the three-qualifying threshold, and the nightly wiring.\n\n## Evidence\n\n| Window (UTC) | Run | Trigger | prediction | floor | nudge | sparse |\ninput ring |\n| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |\n| 2026-11-13 04:11 | 43363930081 | schedule | 40 | 620 | 895 | 0 | 40 |\n| 2026-11-14 04:11 | 43555370869 | schedule | 40 | 617 | 897 | 0 | 40 |\n| 2026-11-15 04:11 | 43675676701 | schedule | 40 | 615 | 897 | 0 | 40 |\n| 2026-11-15 05:32 | 43681492566 | workflow_dispatch | 40 | 614 | 894 |\n0 | 40 |\n| 2026-11-15 05:57 | 43683288193 | workflow_dispatch | 40 | 615 | 898 |\n0 | 40 |\n| 2026-11-15 06:29 | 43685697209 | workflow_dispatch | 40 | 616 | 886 |\n0 | 40 |\n| 2026-11-15 06:56 | 43687685677 | workflow_dispatch | 40 | 620 | 900 |\n0 | 40 |\n\nEvery window is the full nightly workload (1,000 schedules / 5,000,000\nsteps / 9,000 peer\nsessions) seeded from its unique run ID. Windows 4-7 were dispatched on\n`main` at `fc6a5f55` so\nthe seven-window gate could be measured today; the scheduled sequence\nwould otherwise complete on\n2026-11-19. They are the same nightly workflow, redesign, workload, and\nartifact contract as the\nscheduled windows, with independent seeds. The scheduled windows from\n2026-11-16 onward are the\nfirst live gate checks.\n\nChecked-in evaluator output:\n\n```text\nrollback.at_prediction_limit.v1: 7/7 windows (eligible=true)\nfloor_round.lower_floor_consumable.v1: 7/7 windows (eligible=true)\nconnect_status.nudge_sent.v1: 7/7 windows (eligible=true)\nrollback.sparse_earlier_checkpoint_selected.v1: 0/7 windows (eligible=false)\ninput_ring.within_one_slot_of_capacity.v1: 7/7 windows (eligible=true)\nretained: [\"rollback.at_prediction_limit.v1\", \"floor_round.lower_floor_consumable.v1\", \"connect_status.nudge_sent.v1\", \"input_ring.within_one_slot_of_capacity.v1\"]\n```\n\n## Verification\n\n- `cargo fmt --check`\n- `cargo clippy --workspace --all-targets --features\nhot-join,tokio,json,sync-send,paranoid,trace-validation,violation-census\n-- -D warnings`\n- `cargo tfull` — 3,198 passed, 75 skipped\n- `cargo test --test simulation --features hot-join\n'simulation::sometimes_state::tests::'` — 25 passed\n- `python3 -m pytest scripts/tests` — 2,240 passed, 1 skipped\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix`,\n`actionlint`, plan hygiene, agent skills, markdownlint, typos\n\nNo changelog entry: test/CI-only behavior.",
          "timestamp": "2026-11-15T07:37:43Z",
          "tree_id": "a38d2c2e9f5b527359e63566eadf8496c57f4f59",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/8ecd550794978d4be9e3cb580faac0e3a47f114a"
        },
        "date": 1794728987176,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 40143,
            "range": "± 1618",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25345,
            "range": "± 224",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1244,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2767,
            "range": "± 162",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "e4d48fb0224f88d96c98facabb80ed65f67e5f92",
          "message": "fix: bound checksum history, retain rollback-window inputs, single-pass RLE (#427)\n\n## Description\n\nThree independently reproduced issues from a full\ncorrectness/performance sweep, aggregated into one PR.\n\n### 1. Peer-controlled checksum history could grow without bound\n\n`UdpProtocol::on_checksum_report` bounded `pending_checksums` only\nrelative to the incoming, peer-controlled `body.frame`. A strictly\ndescending `ChecksumReport` stream made the `retain` keep every existing\nentry, so each legal 32-byte datagram permanently added a `BTreeMap`\nnode — with desync detection off, the protocol heuristic was the only\nbound, and with detection on the whole map was scanned every\n`advance_frame`.\n\nRed evidence (new test before the fix):\n\n```text\nthread '...checksum_history_stays_bounded_for_descending_frame_reports' panicked:\ndescending reports grew checksum history to 64 (cap 8)\n```\n\nFix: keep the frame-relative retain as the in-order fast path and add an\nunconditional post-insert count cap that drops the lowest frames. Tests\ncover `DesyncDetection::On`/`Off`, `max_checksum_history` 1 and 8, exact\nsaturation, the highest frame surviving, and arbitrary (including\ndescending) report order in the existing proptest.\n\n### 2. Confirmation discarded inputs a clamped rollback still needs\n\n`SyncLayer::set_last_confirmed_frame` discarded every input below\n`confirmed - 1`, while `P2PSession::adjust_gamestate` clamps a\ngossip-lowered disconnect target UP to `rollback_window_floor()`\n(`current - max_prediction`). When the session was not pinned at the\nprediction limit, the re-simulation requested a frame the queue had just\ndiscarded and returned `SynchronizedInputsFailed`; `disconnect_frame`\nstayed armed, so every later `advance_frame` failed too.\n\nRed evidence (new end-to-end test: `current = 12`, `max_prediction = 4`,\nlowered target 2):\n\n```text\npanicked at src/sessions/p2p_session.rs: a clamped disconnect rollback must stay live:\nInternalErrorStructured { kind: SynchronizedInputsFailed { frame: Frame(8) } }\n```\n\nFix: keep normal confirmation discarding through `confirmed - 1`, and\nwhen the rollback target is actually clamped below the prediction-window\nfloor, raise it to `SyncLayer::oldest_retained_input_frame()` — the\nnewest of every live queue's oldest retained confirmed input. In sparse\nmode it advances to the first frame that also has a loadable saved\ncheckpoint. Empty queues (a peer dropped before its first input, or a\nhot-join reactivated slot) do not constrain that floor; they are served\nby frozen/default or reactivation-floor semantics. The rollback then\nre-simulates from the oldest available history and clears\n`disconnect_frame`. The sparse checkpoint walk only commits to a\nloadable frame strictly before `current_frame`; if none exists, the\nrollback fails closed rather than silently clearing the armed\ndisconnect.\n\nAn initial attempt that globally retained the window floor caused a\ndeterministic hot-join participant-loss regression (`Running` vs\n`Synchronizing` in `npeer_no_desync_battery_survivor_drop_variants`), so\nthat approach was reverted before this PR head. The new test verifies\ntwo successful advances, a cleared `disconnect_frame`, and a real\nre-simulation;\n`incomplete_resimulation_does_not_record_completed_rollback` now forces\na genuinely missing-input failure via\n`truncate_input_queues_for_test(0)`.\n\n### 3. Single-pass RLE encoding for small network buffers\n\n`rle::try_encode` scanned every buffer twice (exact length pre-scan,\nthen write). Buffers up to 4 KiB now reserve a conservative fallible\ncapacity (`n + ceil(n/2) + 16`) and encode in one pass; larger buffers\nkeep exact two-pass sizing. The write loop is shared, so encoded bytes\nare unchanged. Empty input still returns without allocating. A new\nproptest asserts the capacity bound covers every output up to 4 KiB.\n\nMeasured with `cargo bench --bench compression -- --quick 'RLE\nencode|Compression pipeline'` (criterion comparison against the saved\npre-change baseline):\n\n| Benchmark | Before | After | Delta |\n| --- | ---: | ---: | ---: |\n| `RLE encode/random/256` | ~410 ns | 118 ns | −71% |\n| `RLE encode/zeros/256` | ~293 ns | 103 ns | −64% |\n| `RLE encode/random/64` | ~138 ns | 59 ns | −58% |\n| `RLE encode/random/16` | ~64 ns | 32 ns | −50% |\n| `Compression pipeline/fighting_encode_8b/32` | ~549 ns | 419 ns | −24%\n|\n| `Compression pipeline/active_encode_8b/32` | ~333 ns | 270 ns | −18% |\n\nNo changelog entry for the RLE-only performance change: the enforced\nunreleased-code rule rejects non-breaking `Changed` entries while\n`[Unreleased]` has an `Added` section. The two correctness fixes carry\n`**Pre-existing:**` Fixed entries.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove each fix is effective\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly (`ProtocolConfig`\nrustdoc, TLA abstraction note, defensive-programming skill rule)\n- [x] I have added `CHANGELOG.md` entries for the user-facing fixes\n- [ ] I have updated relevant examples in the `examples/` directory (not\napplicable)\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `checksum_history_stays_bounded_for_descending_frame_reports` — red\nbefore the fix, covers both detection modes and cap 1/8.\n- `prop_checksum_history_bounded` — now feeds arbitrary report order and\nasserts the cap after every insert.\n-\n`gossip_lowered_rollback_target_resimulates_within_the_rollback_window`\n— red before the fix; two successful advances, a cleared armed rollback,\nand a real re-simulation.\n- `npeer_no_desync_battery_survivor_drop_variants --features hot-join` —\nfail-closed behavior preserved after the final fix.\n- `oldest_retained_input_frame_ignores_empty_queues` —\nempty/dropped/reactivated queues do not collapse the shared floor.\n- `sparse_rollback_without_an_in_window_checkpoint_fails_closed` — no\nsilent skip when sparse has no loadable checkpoint.\n- `incomplete_resimulation_does_not_record_completed_rollback` — forces\nmissing queue state instead of relying on the over-discard.\n- `prop_single_pass_capacity_bounds_output` — one-pass capacity always\ncovers encoded output for arbitrary inputs up to 4 KiB.\n\n**Manual testing performed:**\n\n- Three fresh-seed nightly fleet shards\n(`FORTRESS_SIM_SEED_BASE=20261115`, shards 0/1/4; 375 schedules /\n1,875,000 steps): all green, zero oracle failures.\n- 150-second libFuzzer runs of `fuzz_protocol_input_packet` and\n`fuzz_message_parsing`: exit 0, no crash artifacts.\n- `cargo tfull` — 3203 passed, 75 skipped; full `--features hot-join`\nsuite — 3492 passed, 76 skipped; hot-join survivor-drop battery passes.\n- `cargo nextest run -p fortress-rollback --lib network::protocol` — 258\npassed; `--lib rle` — 88 passed.\n- `cargo nextest run --workspace --features tokio,json -E\n'test(allocation_contract)'` — pass.\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix` — all checks\npassed.\n\n## Related Issues\n\nNone; discovered by the deep-analysis sweep of an otherwise empty queue.\n\n---",
          "timestamp": "2026-11-15T20:26:47Z",
          "tree_id": "ded564e095261b75f4afca62165d49529f162a62",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/e4d48fb0224f88d96c98facabb80ed65f67e5f92"
        },
        "date": 1794775158071,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 40414,
            "range": "± 1441",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25171,
            "range": "± 425",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1244,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2780,
            "range": "± 193",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "fae9d40717215859af87db577f24f80227ea493e",
          "message": "perf: trim steady-state per-frame folds and allocations; fix the Kani ProofVec iteration (#428)\n\n## Summary\n\nThree bounded hot-path improvements plus a merge-queue CI fix, with\nred/green evidence for each.\n\n### 1. Main is red: Kani could not compile\n`SyncLayer::oldest_retained_input_frame`\n\nPR #427 added `for queue in &self.input_queues`. `ProofVec<T>` is a real\n`Vec<T>` in every production build but the stack-backed `InlineVec`\nunder `cfg(kani)`, which intentionally exposes only\n`push/get/get_mut/len/first/clone/iter/iter_mut`. The merge-group lanes\nwere the first place `cfg(kani)` compiled, and all Kani tiers failed at\n`src/sync_layer/mod.rs:412` with `&InlineVec<...> is not an iterator`.\n\nFix: iterate through the documented common subset\n(`self.input_queues.iter()`). Swept every `ProofVec` field for the same\npattern; this was the only by-reference iteration.\n\nLocal proof evidence (Kani 0.68 bundle; the local 0.67 bundle is empty,\nso `--cap-lints allow` suppresses a nightly-2026-08-21-only const-assert\nlint that CI's pinned 0.67 toolchain does not emit):\n\n```text\ncargo kani --harness proof_frame_new_valid                      -> 0 of 3 failed, SUCCESSFUL\ncargo kani --harness proof_minimal_sync_layer_initial_state_valid_2p -> 0 of 448 failed (4 unreachable), SUCCESSFUL\n```\n\n### 2. `confirmed_frame` recomputed the relay-topology gate per slot\n\n`P2PSession::confirmed_frame` computed the slot-independent\n`pessimistic_floor_relay_topology` once, then each per-slot\n`remote_slot_confirmed_bound` recomputed it inside\n`remote_slot_confirmed_bound_observation`, rescanning every remote\nendpoint per slot. The gate is now passed in; the wrapper keeps the\nexisting call shape for tests/diagnostics. Pure-predicate reuse, no\nbehavior change.\n\n`cargo bench --bench h16p_mesh -- --quick`, same host and saved\ncriterion baseline:\n\n| N | Before | After | Delta |\n| ---: | ---: | ---: | ---: |\n| 2 | 23.918 ns | 14.980 ns | -37% |\n| 4 | 82.668 ns | 58.861 ns | -29% |\n| 8 | 329.39 ns | 218.36 ns | -34% |\n| 16 | 1.8350 us | 739.11 ns | -60% |\n\n### 3. Idle receive polls reserved a four-message batch\n\n`receive_all_messages_from` (`UdpNonBlockingSocket`, `TokioUdpSocket`)\nreserved four slots before attempting any receive, so an idle poll\nallocated a 704-byte buffer per session per frame. The batch is now\nsized on the first decoded message (four slots then, one per later\nmessage). New allocation contract:\n\n```text\nred:   idle built-in UDP receive poll: allocations: 1, bytes_allocated: 704\ngreen: idle built-in UDP receive poll: 0 allocations\n```\n\n### 4. Spectator serving rebuilt a handle-keyed map every frame\n\n`send_confirmed_inputs_to_spectators` built a `BTreeMap<PlayerHandle,\nPlayerInput<_>>` per confirmed frame per send, and\n`SyncLayer::confirmed_inputs` collected into a heap `Vec`. It now\nreturns a fallibly reserved `SmallVec<[PlayerInput<_>; 4]>` and\n`InputSource` has an indexed-slice implementation documented for the\nascending complete vector `confirmed_inputs` produces. Wire encoding\norder is unchanged.\n\nNew allocation contract (host + one spectator, steady ack polls):\n\n```text\nred:   6 allocation operations / 290 bytes per measured frame\ngreen: 4 allocation operations / 66 bytes, stable across repetitions\n```\n\nThe remaining allocations are the Input message's own compression/queue\nbuffers.\n\nThe checked-in violation-site inventory was regenerated: the removed\nreceive-batch reservation site is gone and the new confirmed-input\nreserve site is registered (`284` entries still match production).\n\n## Verification\n\n- `cargo fmt --check` - clean.\n- `cargo clippy --workspace --all-targets --features\ntokio,json,hot-join` - clean (only the pre-existing `wasm-browser-smoke`\nmanifest warning).\n- `cargo tfull` - **3203 passed, 75 skipped**.\n- `cargo nextest run --features tokio,json,hot-join -E 'test(spectator)'\n--workspace` - 243 passed.\n- `cargo nextest run -p fortress-rollback --lib p2p_session --features\nhot-join` - 253 passed.\n- `cargo nextest run --test allocation_contract --features tokio,json` -\npass.\n- Fresh-seed nightly shard 0 (`FORTRESS_SIM_SEED_BASE=20261115`, 375\nschedules / 1,875,000 steps) - pass, zero oracle failures.\n- Kani tier-1 and tier-2 harnesses above - pass.\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix` - **All\nagent preflight checks passed**.\n- `python3 scripts/hooks/check-changelog-unreleased.py`, markdownlint -\npass.\n\nNo public API, wire format, or behavioral change; the only\nsource-visible change is the internal `SyncLayer::confirmed_inputs`\nreturn type and the internal `InputSource` implementation.",
          "timestamp": "2026-11-15T21:14:35Z",
          "tree_id": "dbbc3651d7fb654854e1af3476f516490f720a19",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/fae9d40717215859af87db577f24f80227ea493e"
        },
        "date": 1794777867562,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 24398,
            "range": "± 115",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 14929,
            "range": "± 159",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 448,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 558,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 5068,
            "range": "± 151",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a18bf91f236e67529c9fd884c00ae34ed65b531c",
          "message": "fix: raise the retained input floor at the window edge; kill mutation survivors (#430)\n\n## Summary\n\nFixes the two mutation-testing reds on main and a real liveness bug at\nthe rollback window boundary that the surviving mutants exposed.\n\n### 1. Real bug: target exactly at the window floor could wedge\n\n`P2PSession::adjust_gamestate` raised the rollback target to the\nretained-input floor only when `frame_to_load < window_floor`. A\ngossip-lowered disconnect target exactly at `window_floor` was left\nuntouched even though confirmation had already discarded inputs below\n`confirmed - 1` and the retained floor sat above it. `load_frame` then\nsucceeded, the re-simulation asked for a discarded input, and\n`adjust_gamestate` returned `SynchronizedInputsFailed` with\n`disconnect_frame` still armed, wedging every later `advance_frame`.\n\nThe branch now runs at `frame_to_load <= window_floor`. Red evidence\nfrom the new regression with `<` restored:\n\n```text\nthread 'sessions::p2p_session::tests::adjust_gamestate_raises_the_retained_floor_at_the_exact_window_edge' panicked:\na floor-edge rollback must stay live: InternalErrorStructured { kind: SynchronizedInputsFailed { frame: Frame(4) } }\n```\n\nGreen: same test loads frame 7 (the retained floor), re-simulates to\nframe 8, and leaves `current_frame == 8`. The sparse checkpoint walk is\nnow a first-loadable range find and has its own scattered-checkpoint\ntest (earlier checkpoint 4, retained floor 6, later checkpoint 7 ->\nloads 7).\n\n### 2. Mutation red on `e4d48fb` (12 missed mutants in the #427 diff)\n\n- `rle.rs` `<=` -> `>`: both branches emit identical bytes, so the new\nallocation contract pins the single-pass conservative reservation\n(`bytes_allocated >= 32`; two-pass allocates 2).\n- `SyncLayer::oldest_retained_input_frame`: restructured to\n`Option<Frame>` + `max`, removing equivalent operator mutants; new\ntwo-queue test with different retained floors returns the maximum (fails\nunder `min`; empty queues still return `NULL`).\n- `adjust_gamestate`: the comparison fixes above remove the boundary and\nboolean mutants; the new edge + sparse tests cover the remaining\ncomparison/find variants.\n\n### 3. Mutation red on `fae9d40` (1 missed mutant in the #428 diff)\n\nThe lazy receive reservation used `reserved ||\nreceived_messages.try_reserve(1).is_ok()`. When the four-slot\nreservation succeeds, the single-slot retry is a guaranteed no-op\nsuccess, so `||` -> `&&` is behaviorally equivalent and unkillable.\nRestructured to an explicit `if/else` retry only on failure. Local\nin-diff mutation run of this branch: **0 missed mutants** after this\nchange (shards: 3 caught / 11 unviable and 1 caught / 12 unviable; the\nonly miss was this now-removed `||`).\n\n## Verification\n\n- `cargo fmt --check` - clean.\n- `cargo clippy --workspace --all-targets --features tokio,json` - clean\n(only the pre-existing `wasm-browser-smoke` manifest warning).\n- `cargo tfull` - **3208 passed, 75 skipped**.\n- `cargo nextest run --test allocation_contract --features tokio,json` -\npass (new RLE single-pass reservation contract).\n- Targeted unit tests for `adjust_gamestate`,\n`oldest_retained_input_frame`, the ordered-slice source, and\n`socket_receive` - pass.\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix` - **All\nagent preflight checks passed**.\n\nTests-only plus internal logic; no wire or public API change. The\nchangelog's existing pre-existing disconnect-rollback fix entry now\ndocuments the floor-edge equality case.",
          "timestamp": "2026-11-15T21:45:59Z",
          "tree_id": "e0131072a3242f2b66e2d1a5173fa6bbee3fd241",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/a18bf91f236e67529c9fd884c00ae34ed65b531c"
        },
        "date": 1794779885911,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60579,
            "range": "± 604",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 32898,
            "range": "± 472",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1862,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 97",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2995,
            "range": "± 180",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1d4376798f7ec24a5a57dce76a87005897b87586",
          "message": "ci(kani): gate PRs on a cfg(kani) compile check (#431)\n\n## Summary\n\nCloses #429, and folds in the actionable findings from three independent\naudits\n(CAP/partition correctness, hot-path performance, adversarial CI\nreview).\n\nPR #427 merged a by-reference `ProofVec` iteration that clippy, tests,\nand rustdoc all\naccept because `ProofVec<T>` is `Vec<T>` in every normal build. The\nfirst compilation of\nthe `cfg(kani)` tree was the merge-group Kani tier, so `main` went red\nafter merging.\nThe PR gate now runs a compile-only `cargo kani --only-codegen` job that\ncatches that\nclass of break before merge.\n\n## CI gate + toolchain\n\n- `.github/workflows/ci-pr.yml`\n- `changes` job runs `scripts/ci/detect-kani-changes.sh` (the single\npath-filter\n    authority). Detection failures fail closed.\n  - `kani-compile` (`needs: changes`) installs pinned Kani and runs\n    `cargo kani --only-codegen`; it is aggregated by `CI Gate`.\n  - Job timeout (40 min) covers the install retry budget plus codegen.\n- `.kani-version` + `.github/actions/setup-kani/action.yml`\n- Single version authority, exact `--version \"=<pin>\"` install, exact\nreported-version\ncheck, shared install/cache steps for the PR gate and all three deep\ntiers.\n  - Devcontainer installs the same pin.\n- Contract tests\n- Executable classifier tests against a scratch git remote (src, nested\nmember\nmanifests, both actions, `.kani-version` => true; docs => false;\nempty/unfetchable\n    base => true).\n- Compile step cannot be bypassed (`continue-on-error`, `|| true`, `;\ntrue` rejected);\n    proof-execution flags are forbidden across the whole PR workflow.\n- The pin must be absent from the composite and every workflow and must\ndrive both\n    cache keys, the install, and verification.\n\n## Correctness / CAP fixes\n\n- **Coordinated drop certificate**: `DropPrepare.participants` is now\nvalidated against\nthe exact current survivor set for every prepare, not just deferred\nones. A single peer\ncan no longer mint a survivor-quorum certificate for an uninvolved third\nslot, and\n  stale-era prepares are rejected after membership changes.\n- **Bounded peer-driven state**: the supersede/abort arms now use the\nsame\n`num_players`-bounded close history as the other three sites (was an\nunbounded deque\n  plus an O(n) replay scan per message).\n- **Spectator final inputs**: a host is tombstoned only after its\ndrained batch is\nprocessed, so a `Goodbye` decoded before the final reliable `Input` no\nlonger drops\n  that input and wedges playback on `PredictionThreshold`.\n- **Sync handshake**: outstanding `sync_random_requests` are capped at\n64 IDs while\n`Synchronizing` (retries continue; a stalled handshake's memory is\nbounded).\n\n## Performance\n\n- `UdpProtocol::last_recv_frame` now uses `BTreeMap::last_key_value` and\nthe history\nprune pops the stale prefix instead of scanning the map on every\npacket/send/poll.\n- `update_player_disconnects` early-outs when no remotes and no live\ncoordinated-drop\nstate exist. Same-run Criterion A/B (`p2p_all_local`, quick, baseline\n`no-guard`):\n  N=2 −9.1%, N=4 −3.7%, N=16 −8.3%.\n\n## Red/green evidence\n\n```text\n# cfg(kani) gate, with `for queue in &self.input_queues` restored\n$ cargo kani --only-codegen\nerror[E0277]: `&proof_vec::InlineVec<input_queue::InputQueue<T>, 7>` is not an iterator\nexit=1\n\n# spectator (deferral reverted)\nspectator_goodbye_before_final_input_preserves_the_input\n  -> last_recv_frame: left NULL, right Frame(0)\n\n# sync retry bound (cap raised)\noutstanding_sync_requests_stay_bounded -> left 1000, right 64\n\n# drop certificate (membership check removed)\ncoordinated_drop_prepare_requires_the_current_survivor_set\n  -> \"a certificate that omits the running survivor must be rejected\"\n\n# drop close history (trim removed)\ncoordinated_drop_closed_history_stays_bounded -> left 1000, right 2\n```\n\n## Verification\n\n- `cargo fmt --all` — clean.\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings` — clean.\n- `cargo tfull` — **3212 passed, 75 skipped**.\n- `cargo nextest run --workspace --features tokio,json,hot-join` —\n**3501 passed, 76 skipped**.\n- `cargo nextest run --test sessions -E 'test(peer_drop)' --features\ntokio,json` — 36 passed.\n- `cargo nextest run --test sessions -E 'test(hot_join)' --features\ntokio,json,hot-join` — 43 passed.\n- `cargo kani --only-codegen` (local 0.68 bundle, `RUSTFLAGS=--cap-lints\nallow`) — pass.\n- `python3 -m pytest scripts/tests` — 2250+ passed (new classifier\nmatrix included).\n- `actionlint`, skill validator, markdownlint, typos, changelog rule —\nclean.\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix` — all checks\npassed.\n- PR #431: `Kani cfg Compile` green; full check list tracked in the PR.\n\nNo public API or wire-format change. The two `**Pre-existing:**`\nchangelog entries cover\nthe spectator ordering fix and the coordinated-drop certificate\nvalidation.",
          "timestamp": "2026-11-15T23:47:57Z",
          "tree_id": "9dbf2c50ef392c1c6c76adf308f87a79e59f8d9c",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/1d4376798f7ec24a5a57dce76a87005897b87586"
        },
        "date": 1794787171713,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 62023,
            "range": "± 870",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 31688,
            "range": "± 220",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1656,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1557,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2894,
            "range": "± 253",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4e6c2e534778e53b538c743bf1f812e822f3506c",
          "message": "fix: bound hot-join pauses, truncate ended spectators, trim receive staging (#435)\n\n## Summary\n\nCloses #433, #434, and #432.\n\n### Spectator terminal truncation (#434)\n\n`FortressError::SpectatorStreamEnded { frame }` is the explicit terminal\nstate for a spectator whose\nlast host disconnected before the next frame's input arrived. The\nspectator still drains every frame\nreceived before the `Goodbye`; a final input arriving in a later poll\nfrom the removed host is\nrejected as an unknown source. Exhaustive error matches need the new arm\n(breaking).\nRed/green: new\n`spectator_late_final_input_after_host_removal_reports_terminal_truncation`;\nthe two\nall-hosts-drained tests now assert `SpectatorStreamEnded` at the exact\ntruncation frame.\n\n### Hot-join pause bound (#433)\n\nEach timed-out serve records a per-handle retry floor. The next serve\nmay open only after the host\nhas advanced `1, 2, 4, ...` frames (cap 256), so a peer that syncs but\nnever acks cannot keep the\nhost paused in a near-continuous duty cycle. The backoff is shared by\nthe 2-peer and N-peer serve\npaths, cleared on successful join or a fresh endpoint era, and only\nduring the backoff window are\nsimulation inputs withheld from the abandoned reserved endpoint\n(preventing its `pending_output`\noverflow from resetting the budget). Normal reserved-slot input traffic\nis unchanged.\nRed/green:\n`hot_join_timed_out_serves_are_bounded_by_exponential_backoff` drives 12\nforced timeout\nwindows and asserts exact `1,2,4,...,256` progress between serves,\nblocked immediate re-requests,\nand a single-window send-queue high-water bound.\n\n### Receive staging (#432)\n\n- Decoded input frames and per-player input vectors use `SmallVec`\ninline storage for the common\n  `<=4` case (fallible heap growth above it).\n- Consecutive packets reuse the connect-status `Arc<[ConnectionStatus]>`\nwhile gossip is unchanged.\n- `P2PSession::poll_remote_clients` retains its event-staging queue\nacross polls.\n- Warmed one-Input receive poll measured: **8 -> 6 allocation ops** and\n**1408 -> 1152 bytes** at\nN=2 (N=4 1424->1168, N=16 1520->1264). `tests/allocation_contract.rs`\nnow enforces 6 ops / 1280\n  bytes and tightens the send byte ceilings to 96/112/500.\n- The suggested `confirmed_frame` gating was implemented and measured,\nthen rejected: it shifted the\n`4p-loss15-rtt200-4b` deterministic sweep baseline from 7119.4 to 7500.7\nbytes/player/sec for a\n  sub-microsecond fold saving. The sweep is green after reverting it.\n\n## Verification\n\n```text\ncargo fmt --all -- --check                         clean\ncargo clippy --workspace --all-targets --features tokio,json,hot-join -- -D warnings  clean\ncargo nextest run --workspace --features tokio,json,hot-join --no-fail-fast            3504 passed, 76 skipped\ncargo tfull                                                                            3214 passed, 75 skipped\ncargo nextest run --test sessions -E 'test(hot_join)' --features tokio,json,hot-join   43 passed\ncargo nextest run --test allocation_contract --features tokio,json                     pass\npython3 scripts/ci/agent-preflight.py --full --auto-fix                                all checks passed\npython3 scripts/hooks/check-changelog-unreleased.py                                    pass\npython3 scripts/hooks/check-plan-hygiene.py                                            pass\n```\n\nChangelog entries cover the breaking error variant, the hot-join\navailability fix, and the receive\nallocation reduction. `PLAN.md` keeps only the measured-rejected\nfold/send-encode follow-up.\n\n\n### Send-side scratch reuse (#432 continuation, `f673d6c`)\n\n`compression::try_delta_encode_into` backs a doc-hidden\n`try_encode_with_scratch`; `UdpProtocol`\nnow owns the XOR-delta scratch used by both pending-input sends and\nconnect-status nudges. Only the\nqueued RLE output allocates per packet. A new test proves the scratch\npath is byte-identical to\n`try_encode` and does not reallocate once warm. Warmed networked send\nallocation contract:\nN=2 **4 ops/66 B -> 3/58**, N=4 **4/90 -> 3/78**, N=16 **5/482 ->\n4/422**; ceilings tightened to\n3/64, 3/84, 4/440. `sweep_pr_gate` remains green, confirming unchanged\nwire traffic. `PLAN.md` now\ntracks only the measured-rejected confirmation-fold sharing follow-up.\n\n\n### Confirmation-fold sharing (#432 completion, `2dfb7fe`)\n\n`poll_remote_clients` already computes the mesh `confirmed_frame` fold\nfor the gossip-liveness\nnudge. `advance_frame` now reuses that exact value when no endpoint\nevent or disconnect propagation\nchanged a folding input; event handling, live coordinated-drop state,\nand hot-join orchestration\ninvalidate the cache, and `update_player_disconnects` reports whether it\nmutated status. A test-only\nfold counter plus\n`advance_frame_reuses_the_poll_confirmation_fold_when_no_state_changed`\nproves an\nidle advance performs **one mesh fold instead of two**. `sweep_pr_gate`\nremains byte-identical.\n`PLAN.md` is now empty.",
          "timestamp": "2026-11-16T03:24:30Z",
          "tree_id": "435a67dc69b3154f427229b35cd73b476ace902f",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/4e6c2e534778e53b538c743bf1f812e822f3506c"
        },
        "date": 1794800195592,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 37586,
            "range": "± 466",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24615,
            "range": "± 214",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1246,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2768,
            "range": "± 165",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "41e7476e3db03f54d89237de00a610fa2a5697a4",
          "message": "fix: keep confirmation folds fresh across floor-round prunes; finish #435 review follow-up (#436)\n\n## Summary\n\nFollow-up to #435 (merged as `4e6c2e5`) that closes out its last review\nthread and fixes two\ncorrectness/availability edges found by an adversarial read of the new\nhot-path code.\n\n### Confirmation fold freshness (`drive_floor_round`)\n\n`poll_remote_clients` caches its mesh `confirmed_frame` fold for\n`advance_frame` reuse. A prune is\ndetected in `drive_floor_round`, which runs after that fold and resets\nevery relay's floor-round\nfreshness. In the relay topology a connected relay's contribution is\ncapped at the previously\nconfirmed frame while the round is incomplete, so the cached pre-reset\nfold can exceed the\npost-reset fold. Reusing it confirmed past the re-armed hold — the exact\nconnected-relay reorder\nhazard the round exists to prevent.\n\n`drive_floor_round` now drops the cached fold on a prune in the relay\ntopology. Outside that\ntopology the fold never consumes round-floor state, so steady-state\nreuse is unchanged (including\nsteady relay meshes with no prune).\n\nRed/green:\n-\n`advance_frame_recomputes_confirmation_when_the_prune_rearms_the_floor_round`\n(session-level): red prints `before=Frame(50) after=Frame(100)`, green\nholds at 50.\n-\n`drive_floor_round_drops_the_cached_fold_only_in_the_pruned_relay_topology`\npins both arms\nunder every feature set (verified red under `--features hot-join` too).\n\n### Spectator terminal state with catch-up disabled\n\nWith `catchup_speed == 0` and the spectator behind, `advance_frame`\nattempts no advance and the\nall-hosts-drained terminal check was gated on `frames_to_advance > 0`.\nAfter the last host left,\nthat configuration returned `Ok(empty)` forever. Once no host remains,\nthe terminal drain now\nadvances one buffered frame per call regardless of `catchup_speed` (so\nthe documented \"drain the\nbuffered prefix first\" contract holds for every configuration), then\nreports\n`SpectatorStreamEnded`. While a host remains, zero catch-up still\nattempts no advance.\n\nRed/green:\n`spectator_all_hosts_dropped_with_catchup_disabled_drains_then_reports_terminal_truncation`\n(red without the drain override: `SpectatorStreamEnded { frame: 0 }`\nbefore delivering buffered\nframes; green: frames 0-2 drain, then the terminal error at frame 3).\nThe buffered-drain,\nlate-final-input, and stream-delay terminal tests remain green. Rustdoc,\nchangelog, API contract,\nuser guide, and wiki mirrors describe the drain and the remaining\nhost-present zero-speed\nbehavior.\n\n### Status-snapshot reuse test (#435 review thread)\n\n`on_input_reuses_status_snapshot_across_unchanged_packets` exercises the\npacket path: two packets\ncarrying new frames with identical status arrays share one snapshot\n(`Arc::ptr_eq`), and a changed\n`last_frame` allocates a new snapshot without mutating the earlier\nevent's contents. Forcing the\ncache-compare branch off makes it fail.\n\n### Mutation survivors from main's post-#435 diff run\n\nThe post-merge diff-scope mutation run on `4e6c2e5` reported nine\nsurvivors in that PR's new\ncode. This PR kills them without changing behavior:\n\n- `coordinated_drop_may_change_confirmation` extracts the poll-time\ncache-invalidation predicate\n(one assertion per live drop state) so each `||`/`!` arm is\nindependently pinned.\n-\n`update_player_disconnects_reports_no_change_when_already_at_the_fence_cut`\npins the new\nmutation-return value: a receipt already at the committed fence cut\nreports `false`; a lower\n  receipt adopts the cut and reports `true`.\n- `pending_output_len_for_tests_reports_each_queued_input` gives the\nhot-join accessor a\npositive-count control instead of relying on a `<=` bound that passes\nfor any constant.\n\n## Verification\n\n```text\ncargo fmt --all -- --check                                       clean\ncargo clippy --workspace --all-targets --features tokio,json -- -D warnings        clean\ncargo clippy --workspace --all-targets --features tokio,json,hot-join -- -D warnings  clean\ncargo tfull                                                      3223 passed, 75 skipped\ncargo nextest run --workspace --features tokio,json,hot-join     3513 passed, 76 skipped\npython3 scripts/ci/agent-preflight.py --full --auto-fix          all checks passed\npython3 scripts/hooks/check-changelog-unreleased.py              pass\npython3 scripts/hooks/check-plan-hygiene.py                      pass\n```\n\nNo public API change; no wire-format change. `PLAN.md` remains empty.",
          "timestamp": "2026-11-16T04:14:49Z",
          "tree_id": "60193421d9c9961aee3e9caeadd512473432405c",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/41e7476e3db03f54d89237de00a610fa2a5697a4"
        },
        "date": 1794803188911,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 34375,
            "range": "± 981",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 21955,
            "range": "± 496",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 638,
            "range": "± 80",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 843,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 6645,
            "range": "± 136",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "97e16a73ed70abcb54d0909a8e7eaa68b75e4816",
          "message": "test(spectator): pin catch-up boundary cases for mutation coverage (#437)\n\n## Summary\n\nTest-only follow-up to #436 (merged as `41e7476e`). Main's post-merge\ndiff-scope mutation run left one\nsurvivor after #436:\n\n```text\nMISSED src/sessions/p2p_spectator_session.rs:803:53: replace > with >= in SpectatorSession::advance_frame\n```\n\nThe catch-up decision is strict (\"more than `max_frames_behind`\"), so a\nlarger `catchup_speed` must\nnot apply when the spectator is exactly at the threshold.\n`spectator_at_the_catchup_threshold_advances_at_normal_speed`\npins exactly that boundary (red under `>=`: advances `Frame(1)` instead\nof `Frame(0)`).\n\nAlso pins the host-present compatibility half of the new terminal-drain\nbehavior, which previously\nonly had host-drained coverage: with a live host and `catchup_speed ==\n0`, `advance_frame` still\nreturns `Ok(empty)` without advancing. This makes the `hosts.is_empty()`\nbranch mutation-visible\n(red under an always-drain variant: `PredictionThreshold`).\n\nNo production code changes.\n\n## Verification\n\n```text\ncargo fmt --all -- --check                                       clean\ncargo nextest run -E 'test(spectator_at_the_catchup_threshold)|test(spectator_zero_catchup_with_a_live_host)|test(spectator_all_hosts_dropped_with_catchup_disabled)'  3 passed\ncargo tfull                                                      3223 passed, 75 skipped\ncargo nextest run --workspace --features tokio,json,hot-join     3513 passed, 76 skipped\npython3 scripts/ci/agent-preflight.py --full --auto-fix          all checks passed\n```",
          "timestamp": "2026-11-16T04:46:18Z",
          "tree_id": "40c9357f3b3649f7dee501eaaceddcddc107c3ba",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/97e16a73ed70abcb54d0909a8e7eaa68b75e4816"
        },
        "date": 1794805040802,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 62420,
            "range": "± 1275",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 30623,
            "range": "± 2265",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1862,
            "range": "± 111",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3141,
            "range": "± 182",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "42b2fecb2076698cf51a5b9ca2deff6eab4cdc5f",
          "message": "fix(hash): canonicalize integer hashing for WASM/native parity (#438) (#439)\n\n## Summary\n\nWhile investigating #438's Rust-versus-WASM/browser differences, I found\nthat `DeterministicHasher` implemented only `Hasher::write`, so derived\n`Hash` values fell back to the standard library's `to_ne_bytes` integer\nwriters. On `wasm32-unknown-unknown` `usize` and `isize` are four bytes\n(eight natively), and slice/collection `Hash` impls length-prefix\nthrough `write_usize`. Scalar `usize`/`isize` fields and collection\nlengths therefore hashed differently in a browser and natively.\n`compute_checksum` was already safe because bincode is pinned to\nfixed-int little-endian; the `Hash`-trait path was not.\n\n- Override every multi-byte `Hasher` writer with fixed-width\nlittle-endian bytes; canonicalize scalar `usize`/`isize` (and the length\nprefixes std routes through `write_usize`) to 64 bits to match the\ncodec's fixed-int policy.\n- 64-bit little-endian hash values are unchanged; 32-bit and big-endian\nproducers change once for scalar/length inputs (documented as a\n`**Pre-existing:**` changelog fix).\n- Std's primitive `hash_slice` writes slice *elements* as native memory\nbytes, bypassing the integer writers, so `[usize]`/`[isize]` element\npayloads still diverge. All docs now narrow the guarantee accordingly\nand direct machine-width collections to `checksum::compute_checksum`.\n- Add `fortress_replay_fixture::check_machine_width_parity()` covering a\nlength-prefixed slice, scalar `usize`, scalar `isize`, a `u32` slice\n(fixed-width elements), and a `usize`/`isize`/`Vec<u32>` state checksum,\nwith native constants. It runs in the Chromium browser lane and the\nGodot Emscripten lane; native tests anchor the same constants plus the\nexact serialized byte layout.\n- The PR gate now compiles `wasm-browser-smoke` for wasm32. Previously\nthe PR gate never compiled the browser test crate; only the merge-queue\n`ci-rust` lane did.\n- Determinism-model doc and wiki mirror: pointer-width row,\nprimitive-collection caveat, endianness mitigation, and WASM platform\nstatus backed by the checked-in lanes.\n\n## Evidence\n\nRed on wasm32 (Node `wasm-bindgen-test-runner`, pre-fix public API):\n\n```text\nslice length prefix: left 0x458ddd9f0d490432 (32-bit prefix) != right 0x01ef76d429b11552 (native)\nusize scalar:        left 0x6d3572669b2cde42 (32-bit)        != right 0x4bd7a317074c5b62 (native)\n```\n\nPost-fix cross-target measurements:\n\n```text\nu32 slice [1, 0x12345678]: 0x659321e6beec9d26 on both targets (covered fixed-width collection case)\nusize slice [7]:           wasm32 0xbf2981316c0c6f53 vs native 0x1a2742e842fb0203\n                           (documented limitation; std bypasses the integer writers)\n```\n\n- `cargo tfull`: 3229 passed, 75 skipped.\n- `cargo nextest run -p fortress-replay-fixture`: 5 passed.\n- `cargo check -p wasm-browser-smoke --target wasm32-unknown-unknown\n--lib --tests` and clippy with `-D warnings`: clean.\n- `cargo check --target wasm32-unknown-unknown --no-default-features`\n(with and without `hot-join`): clean.\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix`: all checks\npassed.\n- Public API census (2756 symbols), doc links, doc claims, markdownlint,\nactionlint, plan hygiene, changelog unreleased rule, PR-gate contract\ntests (19 passed): all pass.\n\nRefs #438. This adds the cross-target parity guard and fixes the\nscalar/length divergence it exposed; the signal-fish zero-wait gate\ndecision remains in Ambiguous-Interactive/signal-fish-server#1616.",
          "timestamp": "2026-11-16T06:36:33Z",
          "tree_id": "0d2f0b91495fddd112dd8c5eb07f75a6259d3ac1",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/42b2fecb2076698cf51a5b9ca2deff6eab4cdc5f"
        },
        "date": 1794811672128,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 25423,
            "range": "± 574",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 16241,
            "range": "± 610",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 476,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 595,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 5449,
            "range": "± 113",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "40e50dd0d4b9dcee2547b7944cc2031a7de9c815",
          "message": "test(ci): execute the determinism suite on aarch64 and 32-bit x86 (#440)\n\n## Summary\n\nAdds a `cross-arch-runtime` CI matrix that **executes** the\ndeterministic suite on the two\nnon-x86_64 architectures the determinism contract depends on:\n\n- **aarch64** under `qemu-user` (the existing `arm64-cross` job only\ncompiles).\n- **i686** natively, exercising the same 32-bit `usize` as `wasm32`.\n\nThe new lanes found and this PR fixes real test-suite defects:\n\n1. **The 32-bit suite did not compile.** `src/rng.rs`'s\n`test_gen_range_usize_large` forms\n`(u32::MAX as usize) + 1000`, a compile-time overflow on 32-bit. Gated\nto 64-bit targets;\n   the >`u32::MAX` range it tests cannot exist on 32-bit.\n2. **Four allocation-diagnostic tests asserted 64-bit `usize::MAX`.**\n`input_queue`,\n`input_bytes::zeroed`, `saved_states`, and `time_sync` expected\n`requested_elements:\nu64::MAX`; a 32-bit target widens `u32::MAX` instead. They now compare\nagainst\n   `u64::try_from(usize::MAX)`.\n3. **The replay byte-bound test was width-specific.** `1_000_000_000 *\nMIN_FRAME_ENCODED_LEN`\noverflows `usize` on 32-bit (\"overflows the byte bound\") but only\nexceeds the remaining\nbytes on 64-bit (\"exceeds ...\"). The test accepts either rejection form\nwhile still\n   requiring the `frames` field to be named.\n4. **The F17 reorder determinism guard was not deterministic off\nx86_64.** The 10-run guard\nfailed under aarch64 QEMU because the choreography used the default\nwall-clock protocol\ntimer: retry/pending-output timing changed which packets were sent\nduring the hold/release\nwindow on a slower host. It now injects `TestClock` (advanced 16 ms per\npoll) and fixed\n   protocol seeds, per the repository's protocol-test rule.\n\n`docs/specs/determinism-model.md` (and the wiki mirror) now records\naarch64 Linux and i686\nLinux as ✅ Verified with the exact CI scope. A CI contract test pins\nboth lanes, their\ntargets, runner/linker, and filters.\n\n## Evidence\n\n- **i686 native**, full workspace minus simulation/multi-process:\n  `2812 tests run: 2812 passed, 490 skipped` (exit 0).\n- **aarch64 QEMU**, workspace minus simulation/multi-process/compression\nproperty tests:\n  `2714 tests run: 2714 passed (4 slow), 590 skipped` (exit 0, 236 s).\n- aarch64 QEMU F17 regression + determinism guard: 2 passed (guard = 10\nruns in 7.6 s).\n- x86_64 targeted F17 tests and the five allocation/byte-bound tests:\npass.\n- `cargo tfull`: 3,229 passed / 75 skipped.\n- `cargo fmt`, `cargo clippy --workspace --all-targets --features\ntokio,json`, `actionlint`,\n122 CI contract tests, `agent-preflight --full`, plan/link/doc-claim\nchecks: clean.\n- `cargo mutants` on `hash` + `time_sync`: 52 mutants, 32 caught, 18\nunviable, 2 timeouts,\n  **0 missed**.\n- `fuzz_replay_decode`: 2,155,102 runs / 91 s, no crash.\n\n## Notes\n\n- The QEMU lane excludes proptest-heavy compression property tests (they\nexceed the native\nper-test budget under emulation; they still run on the native matrix)\nand the nightly\nsimulation/multi-process tiers, matching the `cargo tf` scope. The\ndedicated\n`ci-cross-arch` nextest profile keeps zero retries and allows a 240 s\nceiling for genuinely\n  slow QEMU tests.\n- No production behavior changes; this hardens the cross-platform\nverification surface that\n  guards the WASM/ARM pointer-width fixes from #439.\n\n## CI validation\n\nA `ci-rust` workflow dispatch on the PR head (`5a3ca32`, run\n\n[43823381040](https://github.com/Ambiguous-Interactive/fortress-rollback/actions/runs/43823381040))\ncompleted both new jobs successfully (2,812 i686 tests and 2,808\naarch64/QEMU tests, all passed):\n[Cross-Arch Runtime\n(i686-32bit)](https://github.com/Ambiguous-Interactive/fortress-rollback/actions/runs/43823381040/job/133151399692)\nand\n[Cross-Arch Runtime\n(aarch64-qemu)](https://github.com/Ambiguous-Interactive/fortress-rollback/actions/runs/43823381040/job/133151400218).",
          "timestamp": "2026-11-16T19:16:05Z",
          "tree_id": "008a40c308ec2e26933803aecbec524921165425",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/40e50dd0d4b9dcee2547b7944cc2031a7de9c815"
        },
        "date": 1794857348586,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 62076,
            "range": "± 720",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 30705,
            "range": "± 189",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1862,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
            "range": "± 63",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3134,
            "range": "± 178",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "5d092b4612dca6bc9581d6772d6fe01d8234b72b",
          "message": "test: verify cross-architecture traces, WASM compression parity, and 1v1 partitions (#441)\n\n## Summary\n\nExtends the cross-platform determinism contract from \"the suite runs on\nother\narchitectures\" to \"the real mesh stack and the wire compression pipeline\nproduce\nidentical bytes everywhere they are expected to\", and fills the missing\n1v1 rows\nin the CAP matrix.\n\n### 1. Cross-architecture simulation trace identity\n\nThe `cross-arch-runtime` lanes execute the workspace suite but skipped\nthe whole\ndeterministic-simulation target, because its nightly shards are long\nrelease-mode loops. That left the session / protocol / SimNet stack\nunexercised\non i686 and aarch64.\n\n`smoke_traces_match_checked_in_cross_architecture_digests` runs three\n`SimConfig::smoke(3)` schedules (seeds `7`, `0x5EED_CAFE`, `41`) and\npins the\ntrace digest and final confirmed frames. The digest folds the\nschema-stable JSON\nof confirmed inputs, session states, game-state checksums, event\nstreams, and\nsimulated wire statistics. The CI filters now select this one simulation\nrow on\nboth lanes, and the Python workflow contract pins the selection.\n\nMeasured before checking in the constants — all three seeds produced\nidentical\ndigests on all three targets:\n\n| target | seed 7 | seed 0x5EED_CAFE | seed 41 |\n| --- | --- | --- | --- |\n| x86_64 | `0x04bf5f66ede9cbfd` | `0xa4a17ae97683cb78` |\n`0x44f403167976508a` |\n| i686 native | `0x04bf5f66ede9cbfd` | `0xa4a17ae97683cb78` |\n`0x44f403167976508a` |\n| aarch64 qemu | `0x04bf5f66ede9cbfd` | `0xa4a17ae97683cb78` |\n`0x44f403167976508a` |\n\n### 2. WASM input-compression parity\n\nReplay playback does not traverse XOR-delta + RLE and the machine-width\nguard\ncovers hashing only, so the pipeline every session input packet uses had\nno\nruntime browser/Emscripten guard. `check_compression_parity()` folds\nboth the\nencode and decode outputs for zero/`0xFF` runs, mixed payloads, the\nempty input,\nand the 31/32, 300, and 16,384-length varint-width boundaries into\n`0x6a92a3417a08652746a08659dd6c6182`. It runs natively (including i686\nand\nqemu-aarch64), in Chromium\n(`input_compression_matches_native_reference`), and\nin the Godot/Emscripten lane (`replay_evidence`).\n\n### 3. CAP matrix: 1v1 partitions\n\n`symmetric_partition_components_fail_closed_without_fork` covered 2/2\nand 2/1\nbut not the most common 1v1 topology. Probe evidence at N=2 with a\npermanent\n`ContinueWithout` partition: both peers end `Synchronizing` at\n`final_confirmed=[95, 95]` around the shared cut, no `Disconnected`\nevent, and\nonly reviewed `EndProgress` diagnostics. Added `vec![0, 1]` to the row;\nthe\nexisting mutation control still flips it to `Forked`. The healed\nquorum-less\nrow now covers `(2, 1)` alongside `(4, 2)` and `(8, 4)`.\n\n### 4. Documentation\n\n`docs/specs/determinism-model.md` referenced a nonexistent\n`cargo run --example replay -- record|verify`. Replaced it with the real\nrecording and validation-mode workflow from `docs/replay.md`, listed the\nCI\nguards that perform the comparisons, and marked x86_64 Windows/macOS\nplus arm64\nmacOS verified with their exact main/merge-queue scope. Wiki mirror\nregenerated.\nThe completed cross-architecture plan item was removed from `PLAN.md`.\n\n## Evidence\n\n- Pinned trace digest: x86_64 pass (0.49 s), i686 pass (0.40 s), aarch64\nQEMU\n  pass (5.78 s) locally; Windows 0.607 s and macOS 0.550 s in CI.\n- Compression digest: x86_64, i686 native, and aarch64 QEMU pass locally\nand in\n  CI.\n- `cargo nextest list` with the new CI filter selects exactly the one\n  simulation identity test.\n- `cargo tfull`: 3,231 passed / 75 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json,hot-join\n-- -D\nwarnings`: clean. Godot nightly clippy `--all-targets --all-features`:\nclean.\n- `cargo test -p wasm-browser-smoke --target wasm32-unknown-unknown\n--lib\n  --no-run`: builds.\n- `actionlint`, 31 CI contract tests, `agent-preflight --full\n--auto-fix`,\n  `check-links`, `check-doc-claims`, `check-plan-hygiene`, `typos`, and\n  `markdownlint`: clean.\n- Latest main (`40e50dd`): all 191 check-runs completed in\n  success/skipped/neutral.\n\n## CI validation\n\nA dispatched `CI - Rust` run on the final PR head (`a18f472`, run\n\n[43968647256](https://github.com/Ambiguous-Interactive/fortress-rollback/actions/runs/43968647256))\ncompleted **success** (23 jobs success, 1 skipped). The full job\nevidence below\ncomes from the code-identical run\n[43948983493](https://github.com/Ambiguous-Interactive/fortress-rollback/actions/runs/43948983493)\non `1fa190e`; the later commits are documentation-only:\n\n- [Cross-Arch Runtime\n(i686-32bit)](https://github.com/Ambiguous-Interactive/fortress-rollback/actions/runs/43948983493/job/133586104549)\n  selected and passed both new fixture rows (2,814 tests).\n- [Cross-Arch Runtime\n(aarch64-qemu)](https://github.com/Ambiguous-Interactive/fortress-rollback/actions/runs/43948983493/job/133586104542)\n  passed both (2,810 tests).\n- [WASM Build Check\n(wasm32-unknown-unknown)](https://github.com/Ambiguous-Interactive/fortress-rollback/actions/runs/43948983493/job/133586104576)\n  reports `input_compression_matches_native_reference ... ok`.\n- [Godot Emscripten\nBrowser](https://github.com/Ambiguous-Interactive/fortress-rollback/actions/runs/43948983493/job/133586104456)\n  Playwright `3 passed`.\n- Build & Test on Windows and macOS both passed the pinned simulation\ndigest.\n\n## Notes\n\n- No production behavior changes; test, CI, and documentation only.\n- This container has no Chromium or Godot/emsdk, so the two browser\ninvocations\nare verified by CI. The digests themselves were verified natively on\nx86_64,\n  i686, and aarch64 before being checked in.",
          "timestamp": "2026-11-16T22:44:02Z",
          "tree_id": "009722d86a9007e86887824a40b9c3f9d1669383",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/5d092b4612dca6bc9581d6772d6fe01d8234b72b"
        },
        "date": 1794869773233,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 41859,
            "range": "± 462",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24200,
            "range": "± 303",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 682,
            "range": "± 86",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 872,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 6827,
            "range": "± 176",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "0c6a97e9facf88c442c386da9dfba11df7f64778",
          "message": "fix: close CAP drop-certificate liveness and rollback-floor/checkpoint defects (#442)\n\n## Summary\n\nDeep correctness/CAP/cross-platform sweep. Fixes a rollback-floor\nliveness defect, two\ncheckpoint/confirmed-input trust defects, two measured checksum/hot-path\nperformance costs, and the browser-wasm `tokio` build break. Adds the\nmissing permanent\nstrict-majority CAP row and the missing clock-skew + healed-partition\nrow, executes checked-in\ncorpus replays on the 32-bit/ARM lanes, and removes a vacuous hot-join\ncorpus guard.\n\n## Production fixes\n\n### 1. In-window rollback below the retained input floor wedged\n`advance_frame`\n\nThe previous floor raise ran only when the target was at/below the\nprediction-window\nfloor. A gossip-lowered target strictly inside the window but below the\ndiscard edge\n(`confirmed - 1`) still loaded a saved state and then failed\n`SynchronizedInputsFailed`\nforever. The target is now raised to the oldest **servable** frame. The\nnew\n`oldest_servable_input_frame` folds retained-ring starts, disconnected\nslots' frozen-value\nwindows, and hot-join reactivation floors, so legitimate sub-activation\nrollbacks keep\ntheir exact load target (both N-peer sub-F hot-join tests stay green).\n\n```text\npre-fix:  Err(SynchronizedInputsFailed { frame: Frame(5) })\npost-fix: loads Frame(7), current_frame stays 8\n```\n\n### 2. Cleared `GameStateCell` was treated as a loadable checkpoint\n\n`save(frame, None, ..)` is a documented clear, but `load_frame`,\n`saved_state_by_frame`,\nand the sparse earlier-checkpoint search tested only the frame stamp.\nAll three now\nrequire stored data; a cleared cell returns `MissingState` instead of\nsilently\nre-simulating from the caller's current state.\n\n### 3. `confirmed_input(Frame::NULL)` fabricated a blank confirmed input\n\n`-1 as usize` wrapped to the last ring slot and could match a blank\n`Frame::NULL` entry.\nNegative frames now return `NoConfirmedInput` before indexing. The\npreviously vacuous\npublic null-frame test now asserts the error.\n\n### 4. Checksum cost\n\n`compute_checksum` / `compute_checksum_fletcher16` now serialize once\nthrough\n`codec::encode_single_pass` instead of a counting pass followed by an\nencoding pass.\n`fletcher16` accumulates in bounded 4096-byte `u32` blocks and reduces\nmod 255 once per\nblock; a property test proves bit identity with the per-byte form across\nthe boundary.\n\n### 5. Hot-join feature tax on plain sessions\n\n`poll_hot_join` cleared the poll-time confirmed-frame cache\nunconditionally, forcing two\nfull mesh folds per frame for every session built with `--features\nhot-join`, even when it\nnever serves or joins. It now clears only when hot-join state exists.\n\n### 6. Browser-wasm `tokio` build\n\n`mio`'s net backend rejects `wasm32-unknown-unknown`, but the `tokio`\nfeature was\nunscoped. The dependency is now target-scoped; the feature is a\ndocumented no-op on\nbrowser wasm and keeps `TokioUdpSocket` on native/WASI/Emscripten. CI\npins the no-op build.\n\n## New CAP / deterministic coverage\n\n- `skew_gated_partition_healed_before_timeout_keeps_one_history` — N=4\n`2/2` partition\nhealed below the disconnect timeout under `FrameModel::SkewGated60Hz`\nwith independent\nper-peer clock rates; all peers Running and within the prediction window\nafterward.\n- Cross-arch CI now executes the checked-in mixed-transport and\nextended-ring corpus\n  replays on i686 and aarch64, not only the pinned smoke trace digest.\n- The hot-join corpus test no longer passes over an empty directory: it\nfalls back to the\ncanonical generated N-peer hot-join schedule and still picks up promoted\nartifacts.\n- Dependency bump: `clap` 4.6.7 → 4.6.8, `clap_lex` 1.1.1 → 1.1.2 (dev\ntooling).\n\n## Validation\n\n- `cargo nextest run --workspace --features tokio,json`: 3238 passed, 75\nskipped.\n- `cargo nextest run --workspace --features tokio,json,hot-join`: 3529\npassed, 76 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json,hot-join\n-- -D warnings`: clean.\n- i686 cross-arch corpus-replay selection: 12 passed (including mixed\ntransport + extended ring).\n- `cargo check --target wasm32-unknown-unknown --no-default-features\n--features tokio`: pass.\n- wasm union\n(`hot-join,json,sync-send,paranoid,trace-validation,violation-census`):\npass.\n- `cargo test -p wasm-browser-smoke --target wasm32-unknown-unknown\n--lib --no-run`: pass.\n- `RUSTFLAGS=\"--cap-lints allow\" cargo kani --only-codegen`: pass.\n- `cd loom-tests && RUSTFLAGS=\"--cfg loom\" cargo test --release`: 9\npassed.\n- `cargo doc --no-deps --features tokio,json,hot-join`: clean.\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix`: all checks\npass.\n- `actionlint`, CI workflow contract tests, changelog hook,\nviolation-site baseline regeneration: pass.\n\n## Follow-ups (not in this PR)\n\n-\n[#445](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/445):\npermanent\nstrict-majority partition with two isolated peers fail-closes the\nmajority; the asymmetric\ntimeout reconciliation needs a protocol-level design and was\ndeliberately reverted from\n  this PR after review.\n-\n[#444](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/444):\ndisconnected\n  relay fresh-floor asymmetry during a poll-only pause.\n-\n[#443](https://github.com/Ambiguous-Interactive/fortress-rollback/issues/443):\nN≥8 warmed\n  `advance_frame` benchmark/allocation row.",
          "timestamp": "2026-11-17T02:56:37Z",
          "tree_id": "e0e306277f09a129bcbdf314b6d400f5572d93d5",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/0c6a97e9facf88c442c386da9dfba11df7f64778"
        },
        "date": 1794884906545,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 40360,
            "range": "± 1148",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25942,
            "range": "± 712",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1244,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2772,
            "range": "± 170",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "d5370e1d6881480527003c8880b75ee6118f4b87",
          "message": "fix: close #442 mutation survivors, fold poll-only relay floors, and budget N≥8 mesh frames (#449)\n\n## Description\n\nMain CI was red on Mutation Testing after #442 (17 missed mutants in the\nnewly added paths). This PR closes those survivors with tests, fixes the\n#444 poll-only disconnected-relay floor, adds the #443 warmed N≥8 mesh\n`advance_frame` budget, and starts the #446 vacuous-test sweep. The #445\nstrict-majority certificate experiment was explored and reverted\n(design/blocker recorded in PLAN) rather than shipping an unproven\nreconciliation.\n\n### Mutation Testing (main CI red)\nMain run `44006997272` failed `Mutation Testing Summary` with **17\nmissed mutants**, all in #442's new code:\n- `oldest_servable_input_frame`: empty reactivation window\n(`frozen_bound == activation_frame`) imposes no floor.\n- `poll_hot_join`: extracted\n`HotJoinState::carries_orchestration_state`; a table test makes each of\nthe nine operands independently significant.\n- `adjust_gamestate`: retained-floor equality/raised/above-floor rows\nwith sparse checkpoint gaps.\n- `GameStateCell::has_data`: cfg(loom)/cfg(kani) bodies use the\nestablished syntax-only `mutants::skip` marker (not compiled by the\nNextest mutation build); the default body stays mutation-tested.\n\n### #444 — disconnected relay fresh floor during a poll-only pause\nA relay polled without advancing can answer a fresh `FloorRequest` with\na floor below its un-mined `status.last_frame`. The consumer now folds\n`min(status.last_frame, fresh round floor)` for both connected and\ndisconnected relays. The `min` keeps the stale-HIGH reorder guarantee\n(status freeze wins).\n\n### #443 — warmed N≥8 `advance_frame`\n`benches/h16p_mesh.rs` times one warmed full-mesh pass at N=8/N=16;\nconstruction, sync, input staging, and polling run outside the timed\nsection. Local medians: **33 µs (N=8)**, **158 µs (N=16)**. CI enforces\ndocumented absolute budgets (300 µs / 1.5 ms, ~10x headroom) via\n`scripts/ci/check-mesh-advance-budget.py`.\n\n### #446 (first slice)\nRemoved four zero-assertion/ignored-result tests: RLE truncation now\nasserts `Err`; TimeSync negative-frame asserts the ring is untouched;\nthe two empty P2P poll tests assert state/frame preservation and event\nsilence.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy** (no new\n`unwrap`/`expect`/`panic!` in production code; new test code uses\nestablished test allowances)\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly (CHANGELOG + PLAN)\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n- Mutation-survivor regressions: empty reactivation window; every\nhot-join orchestration field; retained-floor sparse-search\nequality/raised/above rows.\n- #444 red/green:\n`remote_slot_confirmed_bound_poll_only_disconnected_relay_consumes_fresh_floor`;\nexisting stale-HIGH unit and end-to-end guards still pass.\n- Bench budget: `scripts/tests/test_check_mesh_advance_budget.py`\n(within, over, missing, workflow contract).\n- #446 slice: RLE truncated decode, TimeSync negative frame, two P2P\nempty-poll tests.\n\n**Validation:**\n- `cargo tfull`: **3240 passed / 75 skipped**\n- hot-join suite: **3533 passed / 76 skipped**\n- clippy, rustdoc, doctests (169 passed), preflight `--full --auto-fix`:\nclean\n- wasm32-unknown-unknown feature-union check: pass; i686 and aarch64\nall-targets checks: pass\n- Bench budget script: N=8 33125 ns / 300000 ns; N=16 156548 ns /\n1500000 ns\n\n## Known out-of-scope red\n\n`.github/workflows/openai-review.yml` (added by #448) fails at startup\nwith zero jobs because the private `Ambiguous-Interactive/.github`\nshared workflow is not reachable from this repository. Evidence and\nacceptance criteria: #450. Fixing it requires shared-repo Actions access\nor an org settings change, which this work is not permitted to make;\nevery required check and every other workflow run is green.\n\n## Related Issues\n\nCloses #444\nCloses #443\nRelates to #446\nRelates to #445\nRelates to #450",
          "timestamp": "2026-11-17T05:16:14Z",
          "tree_id": "9791fb05a3468c2658693f72b50ccfda1fdb59ed",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/d5370e1d6881480527003c8880b75ee6118f4b87"
        },
        "date": 1794893226572,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 25449,
            "range": "± 449",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 15578,
            "range": "± 474",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 458,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 585,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 5621,
            "range": "± 164",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "d8641b4fa0806d4d7d988d9bb2d95bdcce3a8fe8",
          "message": "test: pin reset_prediction and P2P constructor outcomes (#446) (#453)\n\n## Description\n\nContinues the #446 vacuous-test sweep. Two tests that previously\nasserted nothing now pin observable behavior:\n\n- `SyncLayer::reset_prediction`: the old test called\n`synchronized_inputs` and `reset_prediction` and asserted nothing. It\nnow verifies both slots are `Predicted` at frame 1, delivers a\nconflicting real frame-1 input, asserts `first_incorrect_frame == 1`,\nthen resets and asserts every queue's `first_incorrect_frame` is NULL.\n- P2P builder tests: `test_add_more_players` and `test_start_session`\nonly returned `Ok(())`; they now assert the constructed session's\n`num_players` (spectators excluded) and `Synchronizing` state.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n## Testing\n\n**Tests added/modified:**\n- `sync_layer::sync_layer_tests::test_reset_prediction` now asserts the\nmisprediction record and reset behavior.\n- `sessions::p2p::test_add_more_players` / `test_start_session` assert\nplayer count and session state.\n\n**Validation:**\n- targeted nextest: 4 + 3 passed\n- `cargo clippy --workspace --all-targets --features tokio,json -D\nwarnings`: clean\n- `cargo tf`: 2776 passed / 492 skipped\n\n## Related Issues\n\nRelates to #446",
          "timestamp": "2026-11-17T06:04:00Z",
          "tree_id": "fb8a0ee605e023d9852147f173d965cc1f7b4aa6",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/d8641b4fa0806d4d7d988d9bb2d95bdcce3a8fe8"
        },
        "date": 1794896137053,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 42230,
            "range": "± 564",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 26257,
            "range": "± 226",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 682,
            "range": "± 151",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 868,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 6895,
            "range": "± 268",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "e48e758d1a50e8b3a2ab0903d22738b1d9b3af90",
          "message": "test: finish the vacuous-test sweep (#446) (#456)\n\n## Description\n\nCompletes the #446 vacuous-test sweep. A call-aware scanner over `src/`,\n`tests/`, and `loom-tests/` found the remaining no-assert tests after\nsessions 228–229; this PR makes the genuine survivors assert their\nintended behavior:\n\n- **RNG:** recursion counts the custom `RandomValue` implementation and\nproves the nested thread-local `random()` re-entry does not deadlock the\n`RefCell`; entropy seeding asserts a non-constant stream.\n- **Telemetry:** `debug_check_invariants!` ok/fail now assert the\ncollected violations, severity/kind, invariant text, and context suffix;\n`report_violation_to!` with an explicit `None` asserts it does not fall\nback to the thread-scoped collector.\n- **Config:** each Debug rendering must name its type.\n\nThe remaining no-literal-assert tests are classified and not vacuous:\n`?`-propagating constructors, `#[should_panic]` contracts, compile-time\nmacro/API contract tests, helper-driven orchestration whose helpers\nassert, and explicit z3 proof summaries.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n## Testing\n\n**Tests added/modified:**\n- `rng::tests::test_random_value_can_call_random_recursively`,\n`test_seedable_from_entropy`\n- `telemetry::tests::test_debug_check_invariants_macro_ok/_fail`,\n`test_report_violation_to_macro_with_none`\n- `test_all_configs_are_debug`\n\n**Validation:**\n- targeted nextest: 6 passed\n- `cargo clippy --workspace --all-targets --features tokio,json -D\nwarnings`: clean\n- `cargo tf`: 2776 passed / 492 skipped\n\n## Related Issues\n\nCloses #446\nRelates to #445",
          "timestamp": "2026-11-17T07:12:51Z",
          "tree_id": "5526a88feb308949c20152383a326b2f20534faa",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/e48e758d1a50e8b3a2ab0903d22738b1d9b3af90"
        },
        "date": 1794900213365,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39837,
            "range": "± 812",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25443,
            "range": "± 517",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1243,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2812,
            "range": "± 300",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "ed08d253d849567c4b3a63ac236534535cdffa97",
          "message": "test: pin both debug_check_invariants! contracts (#457)\n\n## Description\n\nFixes the main `CI - Safety & Correctness` failure introduced by the\n#446 sweep. The strict release jobs (`cargo test --release --lib`) build\nwith debug assertions disabled, where `debug_check_invariants!` is a\ndocumented no-op. The fail-path test now pins both contracts:\n\n- debug / `paranoid`: two Critical/Invariant violations with the\ninvariant text and the context suffix;\n- release no-op: no violations reported.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n## Testing\n\nReproduced both strict modes locally:\n- `RUSTFLAGS=\"-D warnings\" cargo test --release --lib\ntelemetry::tests::test_debug_check_invariants` → 2 passed (no-op\nbranch).\n- `RUSTFLAGS=\"-C debug-assertions=on\" cargo test --release --lib\ntelemetry::tests::test_debug_check_invariants` → 2 passed (reporting\nbranch).\n\n## Related Issues\n\nFixes the main safety-gate regression from #446.",
          "timestamp": "2026-11-17T07:40:56Z",
          "tree_id": "87d275a283cac81cecfdaabb19ee6afb878c07ff",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/ed08d253d849567c4b3a63ac236534535cdffa97"
        },
        "date": 1794901909150,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 40108,
            "range": "± 948",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25117,
            "range": "± 534",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1246,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2798,
            "range": "± 166",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "12da2457e66ccc28ee4221027746164bc9c4d5f5",
          "message": "fix: permit strict-majority graceful-drop certificates (#445) (#458)\n\n## Description\n\nImplements the #445 strict-majority graceful-drop reconciliation without\nchanging the wire format.\n\n- **Deterministic coordinator cut:** the lowest participant handle owns\nthe cut. Only it originates a cut; others verify the same cut, so\nconcurrent nodes cannot lock different cuts.\n- **Pre-Ready cut announcement:** the coordinator announces the cut as\nan `Inventory`-shaped report (`cut` set, zero digest, no receipts)\nbefore it can emit Ready. This lets non-coordinators compute the same\nrequired backfill start and prevents the coordinator/donor deadlock\nfound by the staggered-removal tests.\n- **Strict-majority progression:** cut selection and commit require `n/2\n+ 1` of the shared participants. The coordinator omits only locally\nunresponsive participants (pruned or silence-timeout verdict); a\nreachable-but-slow peer is still awaited on the ordinary path.\n- **Omitted/live-target fail-closed:** commit fan-out reaches\nstill-running target endpoints before they are marked non-running. An\nalive target or omitted survivor with no matching certified attempt\nenters `Synchronizing`; a conflicting cut for an already committed\noperation is rejected.\n- **Asymmetric generation view:** a connected target may be one\ngeneration behind the coordinator's just-disconnected view; the commit's\ndisconnect application bumps the local epoch.\n\n## Regressions\n\n- **N=5 `3/1/1` permanent partition:**\n`strict_majority_partition_with_two_isolated_peers_continues` — majority\n`Running` and >200 confirms past the cut, both isolated peers\n`Synchronizing`, no divergence/fork classes, exact trace replay. Pre-fix\nevidence from #445: all five stalled at 74.\n- **Asymmetric A/B/C vs E:**\n`asymmetric_timeout_witness_omission_commits_majority_and_fails_omitted_closed`\n— A/B/C Ready quorum commits while E is connected-but-omitted; E fails\nclosed on the commit.\n- Timing-sensitive D14 expectations adjusted for the extra certificate\nround (automatic-disconnect +20 steps, PCT known-failure schedule +100\nsteps, split-brain negative control now a one-sided `SplitMatch`).\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n## Testing\n\n- `cargo tfull`: **3243 passed / 75 skipped**\n- hot-join suite: **3537 passed / 76 skipped**\n- strict release (`-D warnings`, `-C overflow-checks=on`, `-C\ndebug-assertions=on`): 1868/1868/1872 passed\n- D14/partition matrix: 257 passed\n- clippy, rustdoc, `agent-preflight --full --auto-fix`: clean\n- wasm32 feature union, i686, aarch64 all-target checks: pass\n\n## Related Issues\n\nCloses #445",
          "timestamp": "2026-11-17T09:23:21Z",
          "tree_id": "ac668506d5954de634e38ace86b202ccc12aa4f2",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/12da2457e66ccc28ee4221027746164bc9c4d5f5"
        },
        "date": 1794908138478,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60684,
            "range": "± 1809",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 32238,
            "range": "± 1293",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1681,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3006,
            "range": "± 190",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cf60d1c9d57a28909351e1a52d9a981b28f2304b",
          "message": "test: kill the coordinator commit-gate boundary mutant (#459)\n\n## Description\n\nFollow-up to #458. The local diff-scoped mutation run reported one\nmissed boundary mutant in the new coordinator commit gate\n(`drive_coordinated_drop` all-ready `<`/`==`). The production check is\nrestructured to an explicit `may_commit` branch, and a regression pins\nthe intended CAP behavior:\n\n- with a bare Ready quorum but a still-reachable omitted witness, the\ncoordinator must **not** commit;\n- once the omitted witness is locally unresponsive, the coordinator may\ncommit.\n\nVerified the mutant (`==` → `!=`) fails the new test (`a reachable\nmissing witness must be awaited`). No user-observable behavior change.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n## Testing\n\n- new test + mutant check: pass/fail as intended\n- `cargo clippy --workspace --all-targets --features tokio,json -D\nwarnings`: clean\n- `cargo tf`: 2783 passed / 493 skipped\n\n## Related Issues\n\nRelates to #445",
          "timestamp": "2026-11-17T10:03:48Z",
          "tree_id": "2320d7474d93613bdd213b074309ac1a5f93c021",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/cf60d1c9d57a28909351e1a52d9a981b28f2304b"
        },
        "date": 1794910588673,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60757,
            "range": "± 630",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 33358,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1862,
            "range": "± 107",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 85",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3142,
            "range": "± 185",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "8a4ec54c4db3a63f6d9c6bc9514f29ddc2a8123b",
          "message": "fix: latch confirmed desync in sync_health, build tokio on wasip1, enforce the pacing oracle (#462)\n\n## Summary\n\nThree correctness/coverage fixes found by a CAP + cross-platform audit\nof `cf60d1c`\n(latest main: 190 check-runs, 0 non-green; no open issues or PRs).\n\n### 1. `sync_health`/`is_synchronized` could report `InSync` on a\ndiverged session\n\n`compare_local_checksums_against_peers` consumed a mismatching checksum\nfrom\n`pending_checksums` after emitting `DesyncDetected`, but only advanced\n`last_verified_frame` on a match. The next `sync_health` call therefore\nfell back\nto a stale earlier match and returned `InSync`; `is_synchronized()`\ncould\ngreen-light termination on a permanent confirmed-frame divergence.\n\nFix: latch the earliest confirmed mismatch per peer\n(`checksum_mismatch_frame`) and consult it before the stale-match\nfallback.\n\n- Red/green:\n`sync_health_keeps_reporting_desync_after_mismatch_is_consumed`\n  fails with the latch removed and passes with it.\n\n### 2. The `tokio` feature could not compile on two of three wasm\ntargets\n\nTokio 1.54 rejects the `net` feature on wasm\n(`Only features sync,macros,io-util,rt,time are supported on wasm`), and\nmio's\nnet modules do not build for Emscripten either. The dependency was only\nscoped\naway from browser `wasm32-unknown-unknown`, so `--features tokio` failed\nwith\nTokio/mio compile errors on `wasm32-wasip1` and\n`wasm32-unknown-emscripten`.\n\nFix: scope the dependency and both module gates away from `target_family\n= \"wasm\"`,\ndocument the no-op, and add `wasm32-wasip1` to the WASM CI matrix. The\ntokio\nno-op check now runs on every wasm row.\n\n- Red: `cargo check --target wasm32-wasip1 --no-default-features\n--features tokio`\nand `--target wasm32-unknown-emscripten` both exited 101 before the\nchange;\n  both are green after.\n- Local matrix verified: no-default, `sync-send`, `paranoid`, `json`,\n`hot-join`,\n`tokio`, plus `clippy -D warnings` on wasip1; browser wasm and\nEmscripten both\ncompile with `tokio` as a no-op; native `--features tokio` still builds.\n\n### 3. Pacing-cliff cells never checked the invariant oracle\n\n`pacing_cliff::measure` only read metrics. A divergence, stalled peer,\nor session\nerror in any ablation cell passed silently. It now calls\n`report.expect_pass(&schedule)`; all five pacing rows stay green.\n\n### 4. Smaller items\n\n- `send_confirmed_inputs_to_spectators` starts with an O(1) emptiness\ncheck on\nthe dedicated spectator map instead of an O(players) handle-registry\nwalk every\n  frame.\n- Corrected the concurrent-removal contract in\n`P2PSession::remove_player`\n  rustdoc, the user guide, and the wiki mirrors. The old text promised\ndeterministic serialization; the checked-in CAP row proves\ndifferent-target\nremovals sharing a coordinator fail closed. Applications must serialize\n  removals. (#461 tracks implementing real serialization.)\n\n## Evidence\n\n| Suite | Result |\n| --- | --- |\n| `cargo nextest run --workspace --features tokio,json` | 3250 passed,\n75 skipped |\n| `cargo nextest run --workspace --features tokio,json,hot-join` | 3544\npassed, 76 skipped |\n| `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings` | clean |\n| WASI feature matrix + clippy | clean |\n| `agent-preflight.py --full --auto-fix` | all checks passed |\n| `actionlint`, workflow contract tests | pass (31 tests) |\n| `check-links`, `check-doc-claims`, markdownlint, typos, changelog\nrules | pass |\n\n## Deep-analysis follow-ups (filed, out of scope)\n\n- #460 — lossy retirement quarantine: N=3 seed 428 wedge\n(`ConflictingHistory`\nfail-closed on every survivor) and N=2 target-tail history divergence,\nwith\n  exact harness repros.\n- #461 — concurrent different-target removals cannot both progress, and\na\nretried removed operation reuses its closed `DropOperationId`, so peers\nswallow\n  the retry and the initiator times out into fail-closed.\n- #463 — the post-prune fresh-ack HOLD can be bypassed when a prune is\napplied\n  after `drive_floor_round` in the same advance.\n- #464 — `advance_frame` drops an already-mutated rollback/save request\nbatch\n  when the spectator flush fails (hot-join serve driven by poll-only).\n\nNo production behavior changed outside the two fixes; wire bytes, trace\ndigests,\nand allocation contracts are unchanged.",
          "timestamp": "2026-11-17T22:54:40Z",
          "tree_id": "04c3b1e0d118724878decf588fc8b99b89691a66",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/8a4ec54c4db3a63f6d9c6bc9514f29ddc2a8123b"
        },
        "date": 1794956757472,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 24457,
            "range": "± 386",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 16134,
            "range": "± 680",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 449,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 561,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 5077,
            "range": "± 162",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cc117df56381ef998a0721ab647929f10c4edf4e",
          "message": "test: kill the same-frame checksum-latch boundary mutant (#466)\n\n## RCA\n\nThe merge-group `Mutation Testing Summary` on main `8a4ec54` (PR #462)\nfailed:\n\n```text\nMISSED src/sessions/p2p_session.rs:13207:80: replace < with <= in\n       P2PSession<T>::compare_local_checksums_against_peers\nerror: diff scope has 1 missed mutants\n```\n\nThe line is the new per-peer mismatch latch predicate\n`remote_frame < latched`. No test delivered a second mismatch for the\n**same**\nframe, so the `<`/`<=` edge was equivalent for every exercised input.\n\n## Fix\n\nExtend\n`sync_health_reports_the_earliest_mismatch_regardless_of_arrival_order`:\nafter the earlier frame is latched, deliver a conflicting duplicate\nreport for\nthat same frame. `sync_health` must keep the first observed checksums\nfor that\nframe; with `<=` the latch rewrites the evidence and the test fails.\n\nVerified with the exact surviving mutant:\n\n```text\n$ cargo mutants -f src/sessions/p2p_session.rs \\\n    -F 'replace < with <= in P2PSession<T>::compare_local_checksums_against_peers' --baseline skip\nFound 1 mutant to test\n1 mutant tested in 76s: 1 caught\n```\n\n## Validation\n\n| Check | Result |\n| --- | --- |\n| `cargo nextest run --workspace --features tokio,json` | 3251 passed,\n75 skipped |\n| `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings` | clean |\n| `agent-preflight.py --full --auto-fix` | all checks passed |\n\nTest-only change; no production behavior or wire bytes change.",
          "timestamp": "2026-11-17T23:17:57Z",
          "tree_id": "5632992cff9811cc42b36ca2b8bff68f1bed9152",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/cc117df56381ef998a0721ab647929f10c4edf4e"
        },
        "date": 1794958247162,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 43775,
            "range": "± 1143",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27707,
            "range": "± 416",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 861,
            "range": "± 74",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1008,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 7940,
            "range": "± 153",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "bdfbca5c7407237f0cd8c53f3aac4029cadcfb73",
          "message": "fix: reset late prunes before the confirmation fold and recover stale spectator cursors (#468)\n\n## Description\n\nCloses the two open advance-path correctness issues and one retry defect\nin the coordinated graceful-drop certificate.\n\n**#463 — a prune applied later in the same `advance_frame` bypassed the\npost-prune fresh-ack HOLD.**\n`drive_floor_round` sampled prune transitions before the endpoint poll,\nso a prune applied afterward by a silence-timeout `Event::Disconnected`\nor by the propagated-disconnect fold did not reset any relay's floor\nfreshness. The confirmation fold in the same advance could then consume\na relay's pre-prune `round_floor`, advance confirmation past the true\nfreeze, and discard inputs at `set_last_confirmed_frame`. Prune\ndetection now runs again after `update_player_disconnects`, gated on the\nrelay topology so nothing changes outside it (the next poll still\nlatches there).\n\n**#464 — the spectator flush could discard an already-applied\nrollback.**\nThe spectator flush now runs before any rollback mutation in both\n`advance_frame` and `advance_frame_npeer_paused`, so a flush failure can\nno longer drop the `LoadGameState`/`AdvanceFrame` batch the application\nstill has to execute. A cursor that lags behind a hot-join reopened\nslot's discarded live-era history (or behind an evicted ring entry) is\npermanently unservable; instead of returning `NoConfirmedInput` on every\nlater advance, the host reports the skipped range and resumes at\n`SyncLayer::oldest_servable_input_frame`.\n\n**#461 (partial) — a retried graceful removal was swallowed by the\nclosed-operation history.**\nA retry reuses the generation-derived `DropOperationId`, so peers that\nhad closed a superseded/aborted attempt silently ignored every retry\n`Prepare` and the removal timed out into a permanent fail-closed. A\nclosed identity is now reopened while\n`coordinated_drop_validate_prepare` still holds (target connected,\ngeneration current, survivor set exact); replays of a concluded\noperation still fail that validation and remain ignored. The separate\nconcurrent-disjoint-removal design question in #461 and the N=2\ntarget-tail semantics in #460 remain open — see \"Related Issues\".\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy** (no new\n`unwrap`/`expect`/`panic!` in production code; all new failures are\nreported, not panicked)\n- [x] I have added tests that prove my fix is effective\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json,hot-join -- -D warnings` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated `CHANGELOG.md` for user-facing changes (four\n`**Pre-existing:**` Fixed entries)\n- [ ] I have updated relevant examples in the `examples/` directory (not\napplicable)\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `same_advance_propagated_prune_resets_the_floor_round_before_the_fold`\n— red on base (verified by removing the new call), green with the fix; a\ngossip-propagated prune inside `advance_frame` now holds confirmation at\nthe pre-prune confirmed frame.\n- `spectator_flush_skips_unservable_reactivation_gap_instead_of_wedging`\n— a stale cursor under a reactivation floor drains the frozen window\ninstead of erroring.\n- `closed_operation_is_reaccepted_only_while_its_target_still_validates`\n— retry reopened while the target validates; stale replay after the\ntarget dropped stays ignored.\n- Existing hand-join spectator/fold tests\n(`npeer_reopened_survivor_with_spectator_flushes_pre_activation_frames`,\n`npeer_joiner_with_own_spectator_flushes_contiguously_from_snapshot_frame`,\n`advance_frame_recomputes_confirmation_when_the_prune_rearms_the_floor_round`,\n`drive_floor_round_drops_the_cached_fold_only_in_the_pruned_relay_topology`)\nall pass unchanged.\n\n**Validation evidence:**\n\n- `cargo nextest run --workspace --features tokio,json`: 3253 passed, 75\nskipped.\n- `cargo nextest run --workspace --features hot-join,tokio,json`: 3548\npassed, 76 skipped.\n- `python3 scripts/ci/agent-preflight.py --full`: exit 0.\n- `cargo check --target aarch64-unknown-linux-gnu\n--no-default-features`: pass.\n- `cargo check --target wasm32-unknown-unknown --no-default-features\n--features hot-join`: pass.\n- `cargo check --target wasm32-wasip1 --no-default-features --features\nhot-join,tokio`: pass.\n- `cargo update`: `bytes` 1.12.1 → 1.12.2 (the only compatible update).\n- `.gitignore`: `mutants.out*/` so local mutation runs leave no\nuntracked state.\n\n## Related Issues\n\nFixes #463\nFixes #464\nRelates to #461 (retry identity fixed; concurrent different-target\nremoval serialization needs a mesh-wide rule and remains open)\nRelates to #460 (lossy retirement/target-tail design decision remains\nopen)",
          "timestamp": "2026-11-18T00:42:09Z",
          "tree_id": "d92e4a8ec6271c59d3a8701785fc3677596c544c",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/bdfbca5c7407237f0cd8c53f3aac4029cadcfb73"
        },
        "date": 1794963229513,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 51390,
            "range": "± 736",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 35058,
            "range": "± 451",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1407,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1604,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3196,
            "range": "± 284",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0980729199572ff5cd51bf6cf52c6214d8a35d46",
          "message": "fix: keep lossy graceful removal live through reordered backfill (#469)\n\n## Description\n\nFollow-up to #468. Closes #460 by fixing the two lossy-retirement\nfailure classes and making lossy graceful removal a green-tier\nsimulation row.\n\n**N=3 certificate wedge (backfill reorder).** A donor's backfill chunk\ncan be delivered ahead of the inventory reports that choose the donor.\n`coordinated_drop_accept_backfill` treated an unattributable chunk as\n`ConflictingHistory` and fail-closed the attempt; the driver also\ndeclared `MissingHistory` as soon as `required_start` was set but no\ndonor was yet visible in the received reports. Both are transient\norderings, not protocol violations:\n- A chunk whose donor is not yet known is ignored; the selected donor\nretransmits every poll while the attempt is active.\n- \"No donor\" is terminal only once every participant's inventory is\nknown (or the missing reporters are locally unresponsive); otherwise the\ndriver waits and the 2s attempt timeout remains the bound.\n\nExact repro (seed 428, N=3, Swarm, `GracefulRemove { by: 2, target: 0\n}`) now keeps both survivors `Running` after the heal with one confirmed\nhistory: `failures=[]`, `final_confirmed=[430, 1398, 1399]`.\n\n**N=2 removed-target tail (design option 1).** A gracefully removed\ntarget can keep confirming its own real inputs for a few frames after\nthe survivors froze its slot. That post-cut tail is private to the\nprocess leaving the match and must not define the canonical exposed\nhistory. The oracle now caps a gracefully removed target's\nconfirmed-input authorship at the survivor-visible receipt cut and its\nstate comparison at `cut + 1`; frames at or below the cut keep full\ncoverage. This is documented in `Oracle::cap_retired_confirmed_inputs`\nand the `dead` field docs.\n\n**Nightly quarantine narrowed.** Non-clean nightly schedules now keep\ngenerated `GracefulRemove` events instead of rewriting them to stalls.\n`PeerKill`/`SpectatorHostKill` remain quarantined: after a kill a 2-peer\nmesh has no strict majority and correctly fails closed, so the\nrecover-after-heal expectation is unsatisfiable there (follow-up issue).\nA sweep of 92 lossy graceful-removal nightly rows (N=2/3/4 ×\nMild/Rough/ReliableFifo/Swarm × seeds 0..40, 5000 steps) passes.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy** (all new failures are\nreported or typed; no new panics in production code)\n- [x] I have added tests that prove my fix is effective\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json,hot-join -- -D warnings` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory (not\napplicable)\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `lossy_graceful_removal_rows_keep_one_history_and_recover` — the exact\n#460 repro seeds (N=2: 133 Rough, 203 Swarm, 273 Mild, 378 Mild; N=3:\n428 Swarm) in the PR-green fleet.\n- `non_clean_nightly_lifecycle_quarantines_only_crash_retirement` —\nreplaces the old all-retirement rewrite assertion; GracefulRemove now\nstays, crash/host-kill retirement remains quarantined.\n- Oracle unit docs/ceilings exercised through the full simulation suite.\n\n**Validation evidence:**\n\n- 92-row lossy graceful-removal nightly sweep (N=2/3/4, 4 noise\nprofiles, seeds 0..40, 5000 steps): 0 failures.\n- `cargo nextest run --workspace --features tokio,json`: 3254 passed, 75\nskipped.\n- `cargo nextest run --workspace --features hot-join,tokio,json`: 3549\npassed, 76 skipped.\n- `python3 scripts/ci/agent-preflight.py --full`: exit 0.\n- Exact repro before/after (seed 428): `ConflictingHistory` fail-closed\n+ survivors stuck at 433/430 → `failures=[]` + 1398/1399.\n\n## Related Issues\n\nFixes #460\nRelates to #461 (concurrent different-target graceful removal\nserialization remains a separate design decision)\nRelates to #468 (previous advance-path batch)",
          "timestamp": "2026-11-18T01:35:21Z",
          "tree_id": "c253505da86024c3a03fcb21eda5c2c950dbc5cf",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/0980729199572ff5cd51bf6cf52c6214d8a35d46"
        },
        "date": 1794966432938,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 35500,
            "range": "± 1472",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 23165,
            "range": "± 970",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 807,
            "range": "± 64",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 841,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 6651,
            "range": "± 107",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "eefebb84cf33f43200593049962f7a1ae94c68a8",
          "message": "test: kill the missing-report disjunction mutant in drive_coordinated_drop (#471)\n\n## Description\n\nMain's merge-group mutation run on `0980729` reported one survivor:\n\n```text\nMISSED src/sessions/p2p_session.rs:3045:21: replace || with && in\n       P2PSession<T>::drive_coordinated_drop\n```\n\nThe condition distinguishes \"reports complete\" from \"every missing\nreporter is unresponsive\":\n`A || B` fails closed when all missing reporters are unresponsive, `A &&\nB` waits forever. No\ntest reached the `A = false, B = true` branch — a missing donor with\nonly unresponsive reporters.\n\nNew test\n`drive_coordinated_drop_fails_closed_when_only_unresponsive_reports_are_missing`\nconstructs that exact state (participants `{0,2,3}`, report from 2,\nendpoint 3 down, cut beyond\nevery receipt) and asserts the terminal `MissingHistory` fail-closed\nplus its diagnostic. Verified\nred on the mutant (`replace || with &&` locally) and green on the fixed\ncode.\n\n## Type of Change\n\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have followed the **zero-panic policy**\n- [x] I have added a test that kills the survivor\n- [x] `cargo fmt && cargo clippy --workspace --all-targets --features\ntokio,json,hot-join -- -D warnings` passes\n- [x] `cargo nextest run` passes\n\n## Testing\n\n- New test passes with the fix and fails with the mutant applied\nmanually.\n- `cargo nextest run --workspace --features hot-join,tokio,json`: 3552\npassed, 76 skipped.\n- `python3 scripts/ci/agent-preflight.py --full`: exit 0.\n\n## Related Issues\n\nFixes the merge-group mutation survivor on #469 (refs #460).",
          "timestamp": "2026-11-18T02:31:04Z",
          "tree_id": "b14fb87ad13e2d9a0d6ebe399868aa3f1aa29db3",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/eefebb84cf33f43200593049962f7a1ae94c68a8"
        },
        "date": 1794969824211,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60687,
            "range": "± 1159",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 31980,
            "range": "± 487",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1723,
            "range": "± 105",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2992,
            "range": "± 213",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "1a674d6c60cd479b841d38b405be8a3509e5bf11",
          "message": "test: un-quarantine lossy crash retirement and pin removal retry commit (#472)\n\n## Summary\n\nUn-quarantines lossy crash retirement in the deterministic fleet (closes\n#470) and records the CAP decision for cross-peer concurrent graceful\nremovals while adding a full retry-commit regression (#461).\n\n### Crash retirement (#470)\n\n- `nightly_schedule` now rewrites `PeerKill` / `SpectatorHostKill` only\nfor the two-peer shape. Every `n >= 3` mesh keeps its real crash event;\na kill leaves a strict majority.\n- `nightly_run_options` uses the production 2 s disconnect timeout\nwhenever a row contains a crash. The inflated modeled-run timeout is\ntransport-only: a crashed peer never resumes, so the inflated timeout\nstarved the automatic drop and froze every survivor at the prediction\nlimit.\n- Added `lossy_crash_retirement_rows_keep_one_history_and_recover`\nacross `PeerKill` and `SpectatorHostKill` under Rough, ReliableFifo, and\nSwarm.\n- Added `two_peer_crash_retirement_fails_closed_without_fork`: the\nexcluded two-peer shape halts in `Synchronizing` at the shared cut with\nno divergence or desync.\n- Narrowed the quarantine test to the two-peer rewrite and assert a\nfive-peer mesh keeps its crash event.\n\n### Concurrent removals and retry (#461)\n\n- Current behavior is retained: cross-peer concurrent different-target\nremovals fail closed. The issue's option 2 (keep the removed target\npolling as an inventory witness until its own commit) was implemented\nand rejected with data. A live target keeps authoring inputs during the\ncertificate; under Rough loss survivors apply different post-request\ntarget frames.\n- seed `12345038` (N=10) reproduced in-band checksum desync at frame\n2130;\n- seed `12345098` (N=10) reproduced in-band checksum desync at frame\n2190;\n  - both rows pass with request-boundary retirement.\n- Added `retried_removal_after_supersede_collects_reports_and_commits`:\naccept op A, supersede with op B, queue/start the retry with a fresh\nidentity, collect Inventory/Ready/Committed reports, commit, and stay\n`Running`.\n\n## Verification\n\n- `cargo tfull` -> 3,260 passed, 75 skipped.\n- `cargo nextest run --release --features\ntokio,json,hot-join,violation-census --test simulation --no-fail-fast`\n-> 484 passed, 27 skipped.\n- Nightly shard 0 at seed base `12345` with `--profile ci --run-ignored\nignored-only` -> 1 passed (163 s), all 125 seeds including the newly\nun-quarantined crash rows.\n- `cargo clippy --workspace --all-targets --features\ntokio,json,hot-join,violation-census -- -D warnings` -> clean.\n- `cargo check --target wasm32-unknown-unknown --no-default-features\n--features hot-join` -> clean.\n- `python3 scripts/ci/agent-preflight.py --auto-fix` -> all checks\npassed.\n\n## CAP notes\n\n- `n == 2` crash retirement has no surviving strict majority (1/2) and\nremains a deliberate fail-closed partition at the shared cut. The row\npins no fork and no desync.\n- Cross-peer concurrent removals remain fail-closed until a protocol\ndesign can keep a leaving target's post-request stream from reaching\ndifferent survivors at different times. The live-witness experiment\nabove is the falsifiable evidence for keeping the halt.\n\n- Follow-up: the live-target desync class found while evaluating option\n2 is tracked in #473.",
          "timestamp": "2026-11-18T07:03:25Z",
          "tree_id": "fbfa00d55eed45f516e89f15f552ca7346d93d6d",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/1a674d6c60cd479b841d38b405be8a3509e5bf11"
        },
        "date": 1794986052120,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 40118,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24588,
            "range": "± 3090",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1245,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2778,
            "range": "± 186",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "646bf83741bedcdcc990b198d81f0de7dba425f4",
          "message": "fix: keep live graceful-removal targets certified without post-cut forks (#474)\n\n## Summary\n\nProduction `remove_player` lets the removed target keep polling and\nauthoring inputs until it observes its own committed certificate. The\nsimulation harness retired the target at the request step, so that\ninterval was never exercised. This PR models the production interval,\nreproduces the post-cut split reported in #473, fixes it, and re-checks\nthe concurrent-removal liveness gap in #461.\n\nCloses #473. Closes #461.\n\n## Root cause\n\nThe certified cut is the **maximum** target receipt across participants.\nA survivor whose own receipt trailed the cut backfilled that frame, but\nit may already have simulated it from a prediction.\n`disconnect_player_at_frames` armed the rollback at `cut + 1`, which\nkeeps the contaminated `state@cut+1` on the speculative side. Nightly\nseed `12345118` (N=15 Rough) shows the split exactly at the first\npost-cut state: peers 0/6 applied the target's predicted value at frame\n1944 while every other survivor applied the certified value; in-band\nchecksums diverged at 1950.\n\nA live target also exposed two receipt-path faults: a retransmission of\na frame the backfill already installed failed the input queue's\nnon-sequential guard and fail-closed the survivor, and benign\nduplicate/late inputs were reported as Error-severity protocol\nviolations.\n\n## Changes\n\n- Re-simulate the certified cut frame itself with the certified value\nwhenever the session already advanced past it, and purge local checksums\nat/above the cut.\n- Advance the receipt high-water when a live target retransmits a frame\nthe backfill already installed; a genuine gap still fails closed.\n- Treat inputs at/below the receipt high-water as benign duplicates at\ntrace level.\n- Add `RunOptions::live_graceful_target`; the nightly fleet enables it\nfor every generated graceful-removal row.\n- Rewrite CAP row 4: two disjoint concurrent removals with live targets\nboth commit in deterministic order (each survivor observes two\n`PeerDropped` events, both targets stop, survivors stay `Running`, no\ndesync).\n- Docs/CHANGELOG describe the certified-cut re-simulation and concurrent\ncertification.\n\n## Verification\n\n- `cargo tfull`: 3261 passed, 76 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json`: clean.\n- `python3 scripts/ci/agent-preflight.py --auto-fix`: all checks pass.\n- Live-target nightly regression\n(`live_target_graceful_removal_under_rough_loss_certifies_without_desync`):\nseeds 12345038, 12345098, 12345118 pass.\n- Nightly shards 0-7 (`FORTRESS_SIM_SEED_BASE=12345`, 125 seeds each)\nwith live-target mode enabled for graceful rows: all green.\n- `live_target_graceful_removal_smoke_keeps_the_mesh_certified` (N=4\nRough) passes in the PR suite.\n- `concurrent_disjoint_graceful_removals_certify_both_without_fork`\npasses with both certificates committed.",
          "timestamp": "2026-11-18T09:10:52Z",
          "tree_id": "572c924d72c4fbfded4847e0ef50d035d1e7a5fb",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/646bf83741bedcdcc990b198d81f0de7dba425f4"
        },
        "date": 1794993813411,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60596,
            "range": "± 581",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 31686,
            "range": "± 180",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1672,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3032,
            "range": "± 267",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "ae8b879232dfb31524d5958ec70d5acdb90e6563",
          "message": "test: pin certified-cut rollback arming and checksum pruning (#475)\n\n## Summary\n\nFollow-up to #474: main mutation CI reported four missed mutants around\nthe certified-cut rollback and checksum purge. This extracts\n`arm_certified_cut_resimulation` so its contract is directly testable\nand adds a unit test that pins:\n\n- the certified cut becomes the rollback target (`disconnect_frame` only\nmoves earlier);\n- local checksums at or above the cut are discarded while earlier\nchecksums remain.\n\nThese assertions kill the checksum-retention and boundary mutants that\nsurvived the diff run.\n\n## Verification\n\n- `cargo tfull`: 3263 passed, 76 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json`: clean.\n- `cargo test --features hot-join --lib\ncertified_cut_resimulation_keeps_only_pre_cut_checksums`: pass.\n\nRefs: #473",
          "timestamp": "2026-11-18T10:04:27Z",
          "tree_id": "18215b446151f7f0755b30e605eea90eeb2b385e",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/ae8b879232dfb31524d5958ec70d5acdb90e6563"
        },
        "date": 1794997029169,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 43676,
            "range": "± 5755",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 28348,
            "range": "± 600",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1183,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1330,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2877,
            "range": "± 160",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b3bf11900471af546e6fea9f90d22f02687b23fe",
          "message": "test: move the cut boundary into the tested arming helper (#476)\n\n## Summary\n\nThe #475 mutation run still reported two missed boundary mutants at the\n`current_frame() > commit.cut` call site: replacing `>` with `==` or\n`>=` either skipped the rollback or armed it at the untested equality\nboundary. This moves the `current <= cut` decision into\n`arm_certified_cut_resimulation` and asserts both sides of the boundary\ndirectly, leaving the caller with no untested branch.\n\n## Verification\n\n- `cargo tfull`: 3263 passed, 76 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json`: clean.\n- Local `cargo mutants --in-diff` against `main`: 0 missed (1 caught, 3\nunviable).\n\nRefs: #473 (#475 follow-up)",
          "timestamp": "2026-11-18T10:36:18Z",
          "tree_id": "5d2918a69da0e7cea4255aff5d2c1691d0157fc4",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/b3bf11900471af546e6fea9f90d22f02687b23fe"
        },
        "date": 1794998860335,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60815,
            "range": "± 1710",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 32041,
            "range": "± 299",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1674,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 107",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3141,
            "range": "± 183",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "bccd0a36971789d26707f996e6735481e81ede6c",
          "message": "docs: qualify concurrent-removal liveness under lossy control (#478)\n\n## Summary\n\nA deterministic Rough 4-peer row with two disjoint `GracefulRemove`\nevents (seed `1129270851`) shows that concurrent removals can still\nwedge a participant that misses the losing certificate's `Superseded`\nabort: it reaches the Inventory timeout and fails closed. This is safe\n(no fork) but qualifies the deterministic-order guarantee added in #474.\n\nChanges:\n\n- User guide and `remove_player` rustdoc now state that disjoint\nremovals commit in deterministic order **while control messages\narrive**, and that a participant which misses the abort under lossy\ndelivery fails closed at the bounded Inventory timeout.\n- CHANGELOG wording qualified the same way.\n- Detailed reproduction and options recorded in #477.\n\n## Verification\n\n- Markdown lint, vale, synced wiki, and agent preflight pass.\n- Doc-only; `cargo tfull` was 3263 passed / 76 skipped before the\nwording change.",
          "timestamp": "2026-11-18T11:31:31Z",
          "tree_id": "ffac6edfc90083f8cf32dc9de8441480d14bc81c",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/bccd0a36971789d26707f996e6735481e81ede6c"
        },
        "date": 1795002237725,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 61347,
            "range": "± 690",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 32990,
            "range": "± 650",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1863,
            "range": "± 203",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3136,
            "range": "± 160",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "778eb1a36d69825094f7d27ab467eb6deec6d36b",
          "message": "fix: serialize concurrent graceful removals through the coordinator (#479)\n\n## Summary\n\nCloses #477. Under Rough loss, two disjoint removals could still\nfail-close a participant of the losing certificate because aborts were\none-shot and non-coordinator prepares opened conflicting attempts on\ndifferent subsets. This makes the deterministic coordinator the single\nadmission point and keeps the losing operation's abort answerable.\n\n## Changes\n\n- Non-coordinator requesters send `Prepare` only to the coordinator;\nother participants ignore prepares from any non-coordinator source.\nConcurrent removals are ordered by the coordinator's admission sequence.\n- Rejected/superseded operations are retained in a bounded (one per\nplayer) journal. Retransmitted prepares and reports for them get another\n`Superseded` abort, so a participant that missed the first abort under\nloss closes before its Inventory timeout.\n- Backfill donor selection falls back to the participant's **current**\nretained range when no start-of-attempt report advertises the full\nrange.\n- Backfill install accepts an evicted, already-confirmed overlap prefix\nwhile still installing the suffix; a still-retained overlap mismatch\nremains a conflict, and a fully stale chunk is a no-op.\n- Live-target premature-exit detection only fires after the target has\nbeen observed `Running` at least once, avoiding false positives for a\ntarget still joining at the removal step.\n\n## Tests\n\n- The live-target nightly regression now includes the deterministic\nRough N=4 concurrent-removal row (both survivors `Running`, both targets\nretired, two `PeerDropped` each) plus seeds `12345333` (backfill\neviction) and `12345758` (target still synchronizing at removal).\n- New unit tests: coordinator-source admission, rejected-operation\njournal, current-retained donor selection, and evicted-overlap backfill.\n- H-RING and the PCT lagging-survivor expectations no longer pin\nunrelated control-plane timing; the capacity fail-closed invariant is\nretained.\n\n## Verification\n\n- `cargo tfull`: 3267 passed, 76 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json`: clean.\n- Agent preflight: all checks pass.\n- Local `cargo mutants --in-diff origin/main`: 0 missed (15 caught, 16\nunviable).\n- Nightly shards 0-7 at `FORTRESS_SIM_SEED_BASE=12345`: all green with\nlive-target mode.\n- Three-seed plus lossy concurrent-removal regression: pass.",
          "timestamp": "2026-11-18T13:36:58Z",
          "tree_id": "9b10577116716f44013f5a1eb1c5b25393534e0e",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/778eb1a36d69825094f7d27ab467eb6deec6d36b"
        },
        "date": 1795009848573,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 41792,
            "range": "± 1508",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25474,
            "range": "± 312",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 683,
            "range": "± 85",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 873,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 6851,
            "range": "± 298",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "f6d42bf589f883b26cdfe865704533f1da47c40d",
          "message": "test: pin rejected-operation journal dedup and bound (#480)\n\n## Summary\n\nFollow-up to #479: main mutation CI reported three missed mutants in\n`coordinated_drop_note_rejected` (the duplicate-operation check and the\none-entry-per-player journal bound). This adds a direct test that:\n\n- re-noting an existing rejected operation does not add a duplicate;\n- distinct rejected operations are retained;\n- inserting more than `num_players` operations keeps exactly\n`num_players`, evicting the oldest first.\n\nManual mutation check: replacing the duplicate check with `!=`, the trim\n`>` with `>=`, and the trim `>` with `==` each makes this test fail.\n\n## Verification\n\n- `cargo test --features hot-join --lib\nrejected_operation_journal_deduplicates_and_stays_bounded`: pass on\nclean source; fails under all three mutants.\n- `cargo clippy --workspace --all-targets --features tokio,json`: clean.\n\nRefs: #477, #479",
          "timestamp": "2026-11-18T14:30:43Z",
          "tree_id": "a327b686e3cb7d0992e4875e9603a63a1f33f439",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/f6d42bf589f883b26cdfe865704533f1da47c40d"
        },
        "date": 1795013034063,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 37876,
            "range": "± 1999",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 22451,
            "range": "± 701",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 641,
            "range": "± 97",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 845,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 6649,
            "range": "± 230",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "62c777a2f81389e56b1014eb290fe4159ca8db19",
          "message": "fix: close coordinated-removal review findings and harness liveness guard (#481)\n\n## Description\n\nAddresses the unresolved review findings from the coordinated\ngraceful-removal work merged in #474, #475, #478, and #479, and improves\nthe certificate's availability under a one-way requester -> coordinator\nfault without reopening split-winner admission.\n\nProduction fixes:\n- A non-coordinator requester broadcasts its `Prepare` to the\noperation's participants. Non-coordinator participants forward it to the\noperation coordinator without admitting it, so a lost\nrequester-to-coordinator link does not stall the removal while a relay\npath exists. Only participants receive prepares; non-participants would\nfail closed on exact-membership validation under asymmetric timeout\nviews.\n- Preserve the original admission source when a prepare is deferred\nbehind a committed certificate instead of recomputing it as the\ncoordinator.\n- Relay a validated coordinator cut announcement through participants,\npreserving the coordinator identity. The first transition and direct\ncoordinator retries are forwarded; relayed duplicates are not\nre-relayed, bounding propagation.\n- Consult the rejected-operation journal before the terminal-operation\nhistory. A superseded operation lives in both collections, and the\njournal is the only path that answers a retransmitted prepare with a\nfresh `Superseded` abort after the first abort is lost.\n- Build the replacement journal payload and reserve it before trimming,\nso a failed allocation cannot evict an entry whose replacement cannot be\nrecorded.\n- Reject a conflicting backfill overlap that only survives in the\nreclaimed rollback-floor slot. `confirmed_input` can still return that\nvalue even when it is not contiguous with the ring, so accepting a\ndifferent backfill value silently contradicted local history.\n\nSimulation/harness:\n- New CAP row 5 `coordinator_link_loss_relays_prepare_and_commits`\n(replay-pinned): peer 2 -> coordinator 0 is blocked from step 100 to 400\nwhile peer 3 is removed at step 120. Direct-only prepares fail closed\nwith `Timeout; phase=Inventory`; the relay commits and all survivors\nrecover after the heal.\n- Record each live removal target's `Running` interval before the\nretirement guard can skip it. The previous write was unreachable, so\n`live_target_premature_exit` could never fire and an early target exit\nwas silently treated as an initial-sync exception.\n- Pin relay and fresh-abort behavior with cumulative per-kind send\ncounters, plus transactional input-queue conflict tests.\n- Require per-target `PeerDropped` evidence in the lossy\nconcurrent-removal row.\n- Remove the broad `coordinated graceful drop` failure allowlist from\nthe H-RING row.\n- Refresh the D14 PCT lagging-survivor expected vector after the\nadditional relay traffic; the regression still has the intended lagging\nshape.\n\nDocs:\n- Qualify the concurrent-removal ordering guarantee per\noperation/coordinator and document the relay path.\n- Document the inventory-failure path, the `Running -> Synchronizing`\nfailed-removal signal (with a bounded initial-sync wait and rebuild\naction), and the journal-retention limit on repeated aborts.\n- Clarify rollback sequencing after a commit and the checksum-history\nbehavior after certified-cut re-simulation.\n- Regenerate the wiki mirrors.\n\nDependencies:\n- `cargo update`: regex 1.13.2, regex-automata 0.4.19, regex-syntax\n0.8.12, smallvec 1.16.6. Workspace lock freshness check passes.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] 🧪 Test (adding or updating tests)\n- [x] 📚 Documentation (changes to documentation only)\n- [x] 🔧 CI/Build (dependency lockfile refresh)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n- `coordinator_link_loss_relays_prepare_and_commits` (red/green CAP row)\n- `superseded_closed_operation_answers_retransmission_with_fresh_abort`\n- `rejected_prepare_is_remembered_and_cannot_reopen` (fresh-abort\nassertions)\n- `non_coordinator_prepare_requires_the_coordinator_source` (relay\nassertion)\n- `reclaimed_floor_overlap_conflict_is_rejected`\n- `evicted_prefix_then_retained_conflict_rejects_transactionally`\n- H-RING allowlist narrowed; lossy removal row requires per-target drop\nevidence; PCT vector refreshed.\n\n**Evidence:**\n- `cargo tfull`: 3272 passed, 76 skipped.\n- `cargo test --doc`: 168 passed, 55 ignored.\n- Targeted\n`coordinator_cut_announcement_requires_exact_shape_and_authority` passes\non native, aarch64, and i686.\n- `cargo test --test simulation`: 471 passed, 28 ignored.\n- Nightly live-target regression: 5 seed rows + lossy concurrent row\npassed.\n- Nightly shards 0-7 at seed base 12345 on `fe245d6`: all green (125\nseeds per shard).\n- Cross-target `cargo check --lib` (default + no-default):\n`wasm32-unknown-unknown`, `wasm32-wasip1`, `wasm32-unknown-emscripten`,\n`i686-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`.\n- `cargo clippy --workspace --all-targets --features tokio,json`: clean.\n- `cargo doc --no-deps --features tokio,json` and link check: clean.\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix`: all checks\npass.\n- `python3 scripts/release/workspace_locks.py check`: passes.\n\nAll review threads are resolved; the latest OpenAI review reports 0\nfindings and every CodeRabbit actionable item has been fixed.\nCodeRabbit's optional full re-review is currently rate-limited, but all\nreported findings are addressed and every review thread is resolved.\n\n## Related Issues\n\nRelates to #473, #474, #475, #477, #478, #479.\n\nFollow-up issues for design-level findings:\n- #482 reliable target-bound commit delivery for a live removed target.\n- #483 receiver-recognized current-history backfill donor.\n- #484 live-target oracle ceiling should use the certified cut.",
          "timestamp": "2026-11-18T20:26:58Z",
          "tree_id": "3b3fb9176c6c35c0868a3fc87d84e7b1a4210894",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/62c777a2f81389e56b1014eb290fe4159ca8db19"
        },
        "date": 1795034331450,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 38131,
            "range": "± 401",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24421,
            "range": "± 414",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1091,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1244,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2782,
            "range": "± 175",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": false,
          "id": "2a1a26c2c90cb727380359df0c4f11ef97e20039",
          "message": "fix: acknowledge live-target removals and attribute current-history donors (#486)\n\n## Description\n\nCloses the three coordinated graceful-removal follow-ups filed from\n#481, and closes the dependency-migration evaluation issue as not\napplicable. All three bugs are in the live-target / lossy-inventory path\nof the D14 graceful-removal certificate.\n\n### Commit delivery to a live removed target (#482)\n\nA survivor disconnects the target endpoint as soon as it applies the\ncertificate, but drop-control datagrams are best-effort and were queued\nexactly once. A live removed target whose copy was lost kept `Running`\non a dropped history until its own peer timeout.\n\n- Survivors now keep a bounded (`<= num_players`) per-target retirement\nfence and retransmit the commit into the endpoint's `Disconnected`\nshutdown window. Retransmission stops on the target's acknowledgement,\nafter a one-second retry window, or as soon as the endpoint shuts down /\nleaves `Disconnected`.\n- A target that receives a committed certificate for an operation it\nnever opened acknowledges with a dedicated `TargetAck` report from its\nsmallest local handle and fails closed. Acknowledgements are advisory:\nthey only stop retransmission and never extend any survivor's confirmed\nfrontier.\n- Duplicate acknowledgements that arrive after the fence cleared are\nignored; they no longer fail the already-committed certificate closed.\n- A hot-join replacement at the same address is a new generation:\nretransmission stops when the endpoint leaves `Disconnected`, and a\nserving host clears the fence when it re-arms the slot. A fresh joiner\nnever receives the departed generation's certificate.\n- A session that has failed closed (`halt_confirmed_ceiling` set) never\ncertifies a removal. A poisoned joiner could otherwise inject a solo\ntarget certificate into a healthy mesh.\n\n### Current-history donor announcements (#483)\n\nInventory reports are start-of-attempt snapshots. A survivor that became\nable to donate afterwards (its missing target frames arrived through\nordinary replication/retransmission) sent chunks that every other\nreceiver discarded, and the attempt failed closed with `MissingHistory`.\n\n- A newly-able donor announces its current retained range as a\n`DropReport` with the new `DropReportStage::Donor` (`cut` = earliest\nservable frame, `cut_digest` = the digest it computes for the certified\ncut). The stage is an additive member of the existing report enum; the\nv3 wire golden registers it.\n- Receivers validate the operation, participant set, agreed cut, and\ndigest, re-derive the canonical chunk count from `[start, cut]` and the\ntarget set, lock one chunk grid per operation, and reject conflicting\nannouncements or chunks instead of mixing donors.\n- A no-donor attempt waits until the two-second operation deadline for a\nlate announcement before failing closed. Inventory completeness\ndescribes the snapshot, not future donation ability, so there is no\nseparate grace period.\n- Inventory-selected donors keep the existing fast path and send no\nannouncement. An inventory-selected donor that stops responding is\nfiltered out, letting a participant that acquired the range after the\nsnapshot announce and supply it.\n- Every backfill chunk carries the producing donor and the start of its\nwhole range. Relays preserve both, and a receiver locks the grid to one\ndonor: chunks from another donor are discarded even when the layout is\nidentical, so two donors cannot interleave earlier inputs.\n\n### Harness oracle fidelity (#484)\n\nFor `RunOptions::live_graceful_target`, the harness capped a target's\nconfirmed inputs at the request-time minimum survivor receipt. The\ncertified cut is the maximum target receipt across survivors, so the\noracle ignored shared history between the two. The cap is now deferred\nto retirement and derived from the certified cut observed on a survivor\n(the request-time receipt remains only a fallback).\n\n`RunReport::donor_announcements` and the doc-hidden\n`P2PSession::diagnostic_donor_announcements_accepted` provide\nnon-vacuity evidence for the donor path.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective\n- [x] `cargo fmt && cargo clippy --workspace --all-targets --features\ntokio,json,hot-join,violation-census` passes with no warnings\n- [x] `cargo nextest run` passes\n\n### If Applicable\n\n- [x] Documentation updated (`docs/user-guide.md`, `remove_player`\nrustdoc)\n- [x] `CHANGELOG.md` entry added under **Pre-existing:** Fixed\n- [x] No new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `committed_drop_retransmits_until_a_live_target_acknowledges`: drops\nthe first commit datagrams at a `MeshBus` transport boundary, asserts\nthe target stays `Running` while it has no certificate, then leaves\n`Running` after the retransmission and asserts the acknowledgement\nclears the fence.\n-\n`committed_drop_retry_state_is_bounded_when_the_target_never_acknowledges`\n- `duplicate_target_ack_after_the_retry_fence_cleared_is_ignored`\n- `donor_announcement_attributes_current_history_backfill`\n- `backfill_chunk_reordered_ahead_of_its_donor_announcement_is_retried`\n- `conflicting_donor_announcements_fail_closed`\n- `backfill_chunks_from_different_start_grids_fail_closed`\n- CAP row\n`reordered_inventory_removal_certifies_without_missing_history` (lossy,\nduplicated, retransmission-delayed N=4 live-target removal,\nreplay-pinned)\n- Harness/oracle regression rows updated for the corrected live-target\ncap (`full_ring_receipt_stagger_fails_closed_at_retained_boundary`,\n`pct_lagging_survivor_does_not_fake_a_freeze_divergence`)\n\n**Validation evidence:**\n\n- `cargo nextest run --workspace --features\ntokio,json,hot-join,violation-census --no-fail-fast` -> 3610 passed, 0\nfailed, 77 skipped.\n- `cargo tfull` -> 3308 passed, 0 failed, 76 skipped.\n- `cargo clippy --workspace --all-targets --features\ntokio,json,hot-join,violation-census -- -D warnings` -> clean.\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix` -> all\nchecks passed.\n- Cross-target checks: `wasm32-unknown-unknown`,\n`wasm32-unknown-emscripten`, `wasm32-wasip1`,\n`aarch64-unknown-linux-gnu`, `i686-unknown-linux-gnu`\n(`--no-default-features --features hot-join`).\n- `cargo update --dry-run`: no compatible dependency updates available\n(the six lagging crates are major-version bumps).\n\n**Review follow-up (CodeRabbit on this PR):**\n\n- Direct donor retransmissions are relayed again (so a lost forwarded\ncopy is repaired) while relayed duplicates are not re-relayed, bounding\npropagation to one extra hop. Pinned by\n`relayed_donor_duplicates_are_bounded_but_direct_retries_relay`.\n- One target acknowledgement now clears every pending retry fence for\nthe same removed endpoint and commit (multi-handle endpoints acknowledge\nonce with their smallest local handle). Pinned by\n`target_ack_clears_every_handle_of_the_removed_endpoint`.\n- Once a donor has announced, the no-donor grace no longer fails the\nattempt; the announced range stays recoverable until the two-second\noperation deadline. Pinned by\n`announced_donor_defers_missing_history_until_the_operation_deadline`.\n\n**Mutation-gate hardening (fixes the current main mutation-summary\nfailure class):**\n\n- Added a table-driven malformed-inventory-report matrix\n(`malformed_inventory_reports_fail_closed_without_mutation`), a\nprepare-routing test\n(`coordinated_drop_send_prepare_targets_only_running_participants`), a\ncommitted-attempt deferral test\n(`prepare_during_a_committed_attempt_is_deferred`), and a matching\nreclaimed-floor overlap control in the input-queue test.\n\n\n**Review follow-up (CodeRabbit round 3):**\n\n- **Wire protocol v3.** The donor/target-ack report stages are\nincompatible with released v2 decoders, and a v2 peer could complete the\nv2 handshake and then drop them. `PROTOCOL_VERSION` is now `3`,\n`MIN_SUPPORTED_PROTOCOL_VERSION` follows it, and\n`src/network/wire_golden_v3.rs` is the active immutable byte suite;\n`wire_golden_v2.rs` is a rejection suite. See the **Breaking:**\nchangelog entry.\n- Target acknowledgements use a dedicated `DropReportStage::TargetAck`,\nso a target (or an omitted session that failed closed) can never be\ncounted as survivor commit evidence. Disconnected endpoints admit only\n`TargetAck`.\n- Multi-chunk backfill validation uses the chunk's explicit\n`range_start` and `donor`, checks the chunk sits at `range_start +\nchunk_index * frames_per_chunk`, locks one donor and range per\noperation, and validates each chunk's frame count. A 16-player two-chunk\nregression pins it.\n- The initial commit is flushed to the retiring target endpoint before a\nhot-join rearm can replace its send queue.\n- A donor may announce an earlier range after a delayed inventory\nreport; the not-yet-installed grid resets instead of failing the honest\nupdate.\n- A rearmed survivor slot clears that address's retirement retries.\n- A failed-closed session closes its uncertified attempt locally without\nemitting witness reports.\n- Racing current-history donors now have a deterministic choice: the\nlowest participant that announced a covering range. Other donors' grids\nare discarded, not failed closed.\n- Diagnostic docs distinguish accepted announcements from completed\nbackfills; the acknowledgement test stays strictly inside the retry\nwindow.\n\n\n**Review follow-up (final rounds):**\n\n- Every `TargetAck` is consumed before the rejected-operation journal,\nso an acknowledgement\nalways clears its retry fence. A multi-handle endpoint's acknowledgement\nis attributed by\n  endpoint address, matching the fence keyed by the removed handle.\n- Announced-donor selection filters unresponsive endpoints; a changed\nselection rebinds the\nuninstalled grid, and before any grid locks the first valid announced\ngrid supplies the range\n  so divergent survivor views cannot discard each other's only chunks.\n- Unseen removal certificates need a second endpoint's corroboration\n(the sole peer suffices in a\ntwo-player session) and must name a coordinator generation the receiver\nhas not moved past; a\nbounded per-identity replay watermark rejects delayed duplicates after\nthe closed journal\nevicts them. A competing known commit needs the same evidence before it\ncan make an active\n  attempt yield.\n- `RunReport` records every installed backfill chunk by donor source;\nthe reordered-removal row\n  asserts a non-vacuous installed suffix.\n- The live-target oracle cap now finalizes at the end of the run so a\nlate-observable certified\n  cut always wins.\n- Validation at `9190785`: `nextest --lib` 1947 passed, `--test\nsimulation` 472 passed, CI profile\n(`--profile ci --features tokio,json`) 3312 passed, `cargo tfull` 3326\npassed, clippy\n`-D warnings` clean, agent preflight passed, wiki dry-run\nbyte-identical.\n\n## Related Issues\n\nCloses #482\nCloses #483\nCloses #484\nRelates to #481, #474, #473\n\n**Review follow-up (CodeRabbit round 8):**\n\n- Commit retransmissions stop as soon as the endpoint leaves the\ndeparted generation's `Disconnected` window. A serving host rearms the\nslot at commit, so the fence clears before any retry and a replacement\ngeneration can never accept the old `DropCommit`. Pinned by\n`hot_join_rearm_stops_commit_retries` and\n`commit_retries_stop_when_the_disconnected_window_closes`.\n- Retirement delivery is deduplicated by address per poll while the\nper-handle fences stay for acknowledgement and cleanup.\n`target_ack_clears_every_handle_of_the_removed_endpoint` also pins a\ndifferent commit at the same address.\n- The donor fallback and the current-history announcement path are\nexercised end to end;\n`lossy_graceful_removal_rows_keep_one_history_and_recover` asserts a\nnonzero `donor_announcements` entry.\n- The no-donor docs name the two-second operation deadline as the only\nterminal verdict, consistent with the earlier Major finding on the\ninventory snapshot.\n\n**Review follow-up (CodeRabbit rounds 9-11):**\n\n- `TargetAck` is consumed before any operation journal, so a committed\nor rejected operation no longer swallows the target's acknowledgement.\nAcknowledgements are attributed by the sender endpoint's address, so an\nack from a co-located handle clears the removed handle's fence for the\nsame commit.\n- Before a grid is locked, the first valid announced grid supplies the\nrange (`selected_announced || no_grid_locked`), so survivors that\ndisagree about the inventory donor's reachability cannot wedge the\nbackfill. Once locked, one donor owns the grid; a changed selection\nrebinds only the uninstalled grid.\n- An unseen certificate needs distinct-endpoint corroboration (two\nendpoints, or the sole peer in a two-player session), a valid\ncoordinator generation, and must be newer than the per-coordinator\nreplay watermark. A single peer can no longer halt an active session;\ndelayed duplicates below a retained cut are rejected.\n- Installed backfills are counted separately\n(`RunReport::backfill_chunks` and `announced_backfill_chunks`); the\nreordered row asserts nonzero installed backfill, and the\nannounced-install path is pinned by\n`donor_announcement_attributes_current_history_backfill`. Generated rows\nstill do not install an announced chunk end-to-end, tracked as #488.\n- The live-target harness cap is finalized at run end: the certified cut\nwins when any survivor observed it, and the request-time ceiling is only\nthe no-certificate fallback.\n- The guide and synced wiki describe the target acknowledgement as the\ndedicated `TargetAck` stage only; no paragraph references a `Committed`\nreport.\n- Closes #487 (ordered `PeerDropped` evidence per survivor).",
          "timestamp": "2026-11-19T07:44:19Z",
          "tree_id": "f05ccff5a5c2a330abd2ab747e7b220b3ce6e146",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/2a1a26c2c90cb727380359df0c4f11ef97e20039"
        },
        "date": 1795074965065,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 24441,
            "range": "± 1296",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 15416,
            "range": "± 385",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 451,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 562,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 5098,
            "range": "± 155",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}