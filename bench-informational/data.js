window.BENCHMARK_DATA = {
  "lastUpdate": 1784392099736,
  "repoUrl": "https://github.com/wallstop/fortress-rollback",
  "entries": {
    "Fortress Rollback Informational Benchmarks": [
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
          "id": "5de83bea40e3a8b32e1d7875d7b3ce997eec6bf7",
          "message": "Harden frame boundaries and benchmark gating (#232)\n\n## What changed\n\n- pin i32 frame saturation and saved-state integrity at the terminal\nframe\n- document checksum-history size-cap retention and cover\nmissing-checksum pruning\n- saturate extreme checksum cadences and retention arithmetic without\nsigned narrowing overflow\n- hard-gate stable microsecond Criterion benchmarks at a 150% threshold\n- keep nanosecond session, input, compression, metrics, and wire-length\ncases informational\n- replace the SyncLayer no-op benchmark with representative save/advance\nwork\n\n## Why\n\nM6 requires deterministic boundary coverage and a performance gate\nstrict enough to catch material regressions without treating\nshared-runner timer noise as a merge blocker.\n\n## Validation\n\n- cargo fmt --all -- --check\n- cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings\n- cargo nextest run --workspace --features tokio,json --no-capture:\n2,866 passed; 73 skipped\n- cargo nextest list --workspace --all-targets --features tokio,json\n- actionlint .github/workflows/ci-benchmarks.yml\n- agent preflight: all checks passed\n- targeted frame, checksum-retention, and extreme-config tests\n- Cursor and Copilot exact-head reviews: zero remaining issues\n\n## Benchmark gate acceptance drill\n\nDraft PR #233 deliberately added 1 ms to Message\nserialization/round_trip_input_msg. Actions run 29214858594 measured\n1,230,741 ns versus the 134,342 ns baseline (9.16x), emitted the\n1.50-threshold performance alert, and failed Run Benchmarks while the\nsmoke job passed. The drill PR was closed without merge and its branch\ndeleted.\n\n---------\n\nCo-authored-by: copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>",
          "timestamp": "2026-07-12T17:27:42-07:00",
          "tree_id": "2109d507ac79456aaaceee75a8390b7e67afa110",
          "url": "https://github.com/wallstop/fortress-rollback/commit/5de83bea40e3a8b32e1d7875d7b3ce997eec6bf7"
        },
        "date": 1783902905062,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1000",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/4",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 279,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 55,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 135,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 455,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 104,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 143,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 168,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 197,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 269,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 361,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 366,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 463,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 687,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 181,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 207,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 267,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 350,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 437,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 553,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 650,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 784,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1024,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 506,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 654,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 879,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1073,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 121,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 172,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 447,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 696,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1049,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 2,
            "range": "± 0",
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
          "id": "7b359e9c66d193081ea31a9c64305451a6ea73a6",
          "message": "Add bounded long-run network soak (#234)\n\n## Summary\n\n- add a deterministic 4,000,000-confirmed-frame nightly soak for\n2-player periodic rejoin and 4-player mild-chaos endurance lanes, both\nwith spectators, replay validation, hard container bounds, high-water\nplateau checks, and Linux RSS gates\n- pre-prune local checksum history so its configured cap is also its\ntrue allocation high-water\n- make repeated hot-join activation loss-safe by deferring pre-commit\ninput processing, backfilling activation frame F, and retrying\nuncaptured N-player serves after honest rollback repair\n- wire the release-mode soak into nightly network CI\n\n## Root cause\n\nThe long-run runner exposed three boundary assumptions that shorter\ntests missed: checksum retention pruned after allocation, a rejoiner's\nactivation input could be consumed or omitted around snapshot commit,\nand an honest pre-capture rollback could move the saved frame while an\nN-player serve waited. The fixes preserve fail-closed behavior while\nmaking these normal loss/reorder cases recoverable.\n\nAn exploratory repeated N=4 generation run also exposed survivor epoch\ndivergence after 20 cycles. That separate D17 remains recorded in\nPLAN.md for a focused follow-up; the committed soak keeps periodic\ngeneration churn in the N=2 lane and runs N=4 as full-duration\nmild-chaos endurance.\n\n## Validation\n\n- full 4,000,000-frame release soak: passed in 203.47 s\n- `cargo clippy --workspace --all-targets --features tokio,json,hot-join\n-- -D warnings`\n- `cargo nextest run --workspace --features tokio,json,hot-join\n--no-capture`: 3,121 passed, 74 skipped\n- `cargo test --doc --features tokio,json,hot-join -- --nocapture`: 169\npassed, 54 ignored\n- `cargo doc --no-deps --features tokio,json,hot-join`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- `actionlint .github/workflows/ci-network-nightly.yml`\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Touches P2P hot-join activation and checksum history in core session\npaths; behavior is narrowed to loss/reorder recovery and bounded\nretention, with a large new soak as regression coverage.\n> \n> **Overview**\n> Adds a **nightly-only** deterministic **4,000,000-frame** network soak\n(2-player periodic hot-join + 4-player mild chaos) that checks replay,\nbounded containers, high-water plateaus, and Linux RSS growth, plus\n**`__internal::p2p_container_lengths`** for those audits.\n> \n> **Hot-join under loss:** the serving host **defers joiner `Input`\nprocessing** until commit, **backfills activation frame F** on\nreactivation, and **aborts uncaptured N-player serves** when rollback\nmoves `last_saved` off the pinned snapshot (warning + retry instead of\nper-poll errors).\n> \n> **Checksum retention:** `check_checksum_send_interval` **prunes before\ninsert** so `max_checksum_history` is a true allocation cap (with unit\ncoverage).\n> \n> **CI:** `ci-network-nightly` runs the soak with `hot-join` and a\ndedicated **600s** nextest override.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n27281e0edae91dd12b90ecad439c048b2f5a7aec. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\n---------\n\nCo-authored-by: copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>",
          "timestamp": "2026-07-12T20:45:25-07:00",
          "tree_id": "bb314924974079c589aaf3756b9790beb290a00b",
          "url": "https://github.com/wallstop/fortress-rollback/commit/7b359e9c66d193081ea31a9c64305451a6ea73a6"
        },
        "date": 1783914778053,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1000",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/4",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 278,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 55,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 136,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 449,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 25,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 104,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 130,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 168,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 182,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 257,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 375,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 345,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 472,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 720,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 181,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 209,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 258,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 350,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 417,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 550,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 650,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 779,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1044,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 486,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 659,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 876,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1088,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 117,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 166,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 443,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 701,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1015,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 2,
            "range": "± 0",
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
          "id": "9bccc10b88e195e645be64a7a990d7fc72683f25",
          "message": "Converge hot-join membership generations (#235)\n\n## What changed\n\n- Separate D14's canonical membership generation from retry-local\nspectator epochs.\n- Carry canonical live/dead membership through N-player replacement\nsnapshots and protect committed cuts across skewed retries.\n- Close unheard reactivation lifecycles before installing a later D14\nfence, with regressions for delayed lifecycle messages and former\njoiners.\n- Add local epoch diagnostics and enable all 40 periodic churn\ngenerations in the N=4 nightly soak.\n- Establish protocol v2 as the exact-match semantic boundary, retain\nreleased-v1 fixtures as rejection coverage, and document the upgrade\nrequirement.\n\n## Why\n\nThe deterministic N=4 soak completed 20 drop/rejoin cycles, then failed\nclosed on generation 21 because survivors derived different D14\ncertificate identities from locally skewed connection-status epochs. A\nreplacement session also lacked the canonical history needed for later\ndrops.\n\n## Impact\n\nRepeated N-player drop/rejoin cycles now converge on one certificate\ngeneration without regressing spectator epochs. Mixed v1/v2 sessions\nintentionally fail closed during raw packet decoding because v1\nsnapshots do not carry the required canonical semantics.\n\n## Validation\n\n- Full hot-join nextest matrix: 3,096 passed, 74 skipped\n- Workspace/all-target cargo tests: passed\n- Historical release soak: N=2 and N=4 through 2,200,000 confirmed\nframes, including generation 21\n- Strict clippy: workspace/all-targets with `tokio,json,hot-join`\n- Strict rustdoc: workspace/all-features\n- Agent preflight, changelog policy, immutable wire-golden hook,\nformatting, and diff checks\n- Six adversarial review/fix passes converged to zero issues\n\n## Review Readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS\n- Determinism: PASS\n- Agent preflight: PASS\n- Error handling: PASS\n- Tests breadth: PASS\n- Design log reviewed: YES\n- CHANGELOG reviewed: YES\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **High Risk**\n> Breaking wire-protocol bump plus changes to graceful-drop\ncertificates, hot-join snapshots, and reactivation/drop interaction—core\nmultiplayer correctness paths that require coordinated fleet upgrades.\n> \n> **Overview**\n> **Protocol v2** is now the active exact-match wire version\n(`PROTOCOL_VERSION` 2); released v1 goldens are kept only as rejection\ntests, and docs/migration call out that **all peers must upgrade\ntogether** because v1 snapshots lack the new membership semantics.\n> \n> **D14 coordinated drops** no longer key certificate generations off\nretry-local `ConnectionStatus::epoch`. A per-slot\n**`membership_generations`** map (updated on commit/reactivation) drives\nprepare/accept/commit checks via `local_coordinated_drop_generation`,\nwhile spectator epochs can still diverge across survivors.\n> \n> **N-player hot-join** normalizes snapshot **`bridge_statuses`** epochs\nto canonical connected-era membership (documented on `StateSnapshot`),\nbuilds snapshots through `snapshot_connect_statuses()`, seeds joiners\nfrom carried statuses, and **closes unheard reopened reactivations**\nbefore installing a later drop fence so delayed `JoinCommitted` cannot\nundo a new fence. Committed-cut shielding no longer requires matching\nspectator epoch.\n> \n> Also: Miri job timeout **30m**, four-player soak churn **enabled** for\nnightly coverage, and expanded regressions around generation-21 churn\nand D17 skew.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n27e370a368cc1fd7970eef070a4e6f9b410a2eb6. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-13T08:55:42-07:00",
          "tree_id": "3a19c1acff4329c8ca9f4dcd39d904c22937a83a",
          "url": "https://github.com/wallstop/fortress-rollback/commit/9bccc10b88e195e645be64a7a990d7fc72683f25"
        },
        "date": 1783958611652,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1000",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/4",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 278,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 55,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 136,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 453,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 105,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 130,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 173,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 181,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 255,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 375,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 345,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 469,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 718,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 181,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 209,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 251,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 349,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 418,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 553,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 651,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 781,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1039,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 497,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 668,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 892,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1102,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 114,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 166,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 450,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 705,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1040,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 18,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 2,
            "range": "± 0",
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
          "id": "a3a1c6d15d6ec58c7f1b7dcb29ccaba37fcf7d5d",
          "message": "Test time-sync controller and stabilize benchmark gate (#238)\n\n## What changed\n\n- add deterministic H-OSC fleet experiments for balanced exogenous\ndelay, policy sweeps, and N={2,8,16} aggregation pressure\n- add schema-v18 bounded CPU-feedback scheduling, replay artifacts,\nshrink axes, and the matched H-META-RB experiment\n- add authoritative direct-receipt/retained-range evidence and close\nH-RING at the full 128-entry fail-safe boundary\n- characterize B2/B4 valid hostile gossip with one-edge mutation,\nreceiver-side cache/floor evidence, bounded recovery, artifact-v7\nreplay, and shrink support\n- document explicit B1/B3/B5/B6 operator dispositions for equivocation,\nfalse checksum accusations, flooding, and snapshot poisoning; correct\nrecovery/attribution overclaims and regenerate wiki mirrors\n- model cooldown, maximum skip, response delay, and smear behavior in\nthe wait-recommendation actuator\n- preserve raw, accepted, and obeyed controller evidence in bounded\nreplay artifacts and extend shrink coverage\n- expose `PeerMetrics::average_frame_advantage` as the exact production\nendpoint statistic\n- wire the aggregation census into the simulation nightly workflow\n- replace cross-runner historical benchmark gating with paired\nbase/merge measurements on one runner\n- repair the scheduled-nightly H-BLOAT scale characterization after\nprotocol-v2 wire growth, pin its two intentional fragmentation alarms\nexactly, and keep A8 time-gated pending one green week\n- retire the false-green `NetworkProtocol.tla` model and seven\ndisconnected protocol-state Kani proofs; reduce the base inventory to 18\nTLA+ modules and 123 registered Kani proofs\n- record a conditional-GO trace-validation decision: first prove a\nno-instrumentation `SyncHandshakeV1` trace contract and mutation before\nadding runtime trace points\n- add the 19th TLA+ module, a mutation-sensitive `SyncHandshakeV1` trace\ncontract: canonical matching/reorder/duplicate, mismatch, and\ntimeout/retry traces pass while an exact duplicate-reply decrement\nmutation fails\n\n## Why\n\nThe current benchmark workflow also compares PR measurements against\nhistorical runs from unrelated runner hardware, producing repeatable\nfalse regressions on unchanged benchmark code. The paired gate measures\nbase and merge revisions on the same runner and requires a majority of\nthree comparisons.\n\nThe H-OSC/A10 milestone requires falsifiable evidence that the\nproduction time-sync controller remains stable under deterministic\nasymmetric timing pressure, that aggregation pressure grows as expected\nwith fleet size, and that the system recovers after the perturbation\nheals. Existing tests did not exercise the production aggregate and\nper-endpoint controller populations on identical evaluation events or\nretain enough policy evidence for exact replay.\n\nH-META-RB also required a deterministic way to charge actual visual plus\nrollback work into future missed peer drives. The matched experiment\nconfirms the modeled capacity edge while finding no runaway cumulative\nresimulation at the tested peer-0-first 8 ms/frame bound; it\ndeliberately does not claim the untested RTT/controller-mediated\namplification path.\n\nH-RING previously relied on an invalid step-count proxy. The new bounded\nprobe measures authoritative connected-observer receipt spread and\nphysical retained history directly. The N=4 entrance row fills\n`F..F+127`, retains `F`, then proves the session-scoped input-capacity\nrefusal enters `Synchronizing` without confirmed-input, state, in-band,\nor checksum divergence.\n\nThe B2/B4 census now mutates only one liar→observer typed-message edge\nand samples the exact accepted receiver cache/floor state. B2 observes\nan exact 12-frame low wedge while an inflated status cannot lift the\nhonest minimum. B4 observes both an exact low wedge and a non-vacuous\nbut bounded high-floor release; it explicitly does not claim coverage of\nan inherited committed-low double-failure choreography.\n\nThe dishonest-peer operator closeout now separates conditional detection\nfrom attribution: checksum disagreement is evidence rather than culprit\nproof, built-in ingress and persistent caps do not cover\ntransient/custom/kernel/DDoS pressure, and hot-join fingerprints do not\nattest snapshot provenance.\n\nThe nightly H-BLOAT row now preserves its queue/fragmentation separation\nat 8.75 KB/s and treats only the exact peer-0↔peer-1 fragmentation\nalarms as premise evidence; capacity, sequence, state, liveness, size,\ndestination, or replay drift still fail. The two A8 N=16 candidates stay\nignored: their warmed 3.133-second combined runtime meets the PR budget,\nbut scheduled main history is one green followed by three failures\nrather than a green week.\n\nThe post-M3 trace-validation audit found that `NetworkProtocol.tla`\ncould not consume a sync request, checked no temporal property, and\nbounded its timers below the disconnect/shutdown guards. It also found a\nhand-written Kani transition table that contradicted production\ndisconnect and hot-join transitions. The obsolete checks are removed;\nreplacement claims are narrowed to the bounded two-peer/two-field\nhandshake model, enum representation checks, and Rust\nproduction-transition tests, with no claimed refinement link.\n\nThe no-instrumentation feasibility gate now passes without overstating\nthe result. A strict NDJSON manifest expands action deltas into complete\npost-action states and constrains them through the real\n`SyncHandshakeV1` actions. The gate owns the exact scenario semantics\nand fails closed on missing/substituted fixtures, malformed schemas,\ntool errors, partial or zero-state searches, output drift, and any\nnegative result other than the intended `EventuallyTraceConsumed`\ncounterexample. Runtime trace production and refinement remain\nexplicitly unimplemented.\n\n## Impact\n\nThe production controller behavior is unchanged. Users gain a documented\nrolling frame-advantage metric. CI gains deterministic replayable\nexperiments and bounded failure artifacts that diagnose controller\nrecommendations, actuator decisions, and receipt/range extrema. Failure\nartifacts advance to v7; the losslessly packed range evidence keeps the\nliteral N=16 × 64-snapshot maximum inside the existing 8 MiB cap, while\nhostile-gossip artifacts preserve mutation and receiver-side diagnostic\nevidence.\n\nFormal-verification CI no longer reports large state counts or proof\ntotals from disconnected models. Targeted TLA output now counts\nunselected checks honestly. The trace-validation pilot has a fast,\nindependently selectable, mutation-sensitive executable contract rather\nthan generic diagnostic snapshots, but still has no production trace\nproducer or runtime refinement claim.\n\n## Validation\n\n- `./scripts/verification/verify-tla.sh SyncHandshakeV1` — 1,586,628\ndistinct states plus the expected handlers-disabled liveness\ncounterexample\n- `./scripts/verification/verify-tla.sh SyncHandshakeV1TraceContract` —\nmatching/mismatch/timeout accepted; exact duplicate-reply decrement\nrejected as required\n- `python3 -m pytest -q scripts/tests` — 1,678 passed\n- `./scripts/verification/verify-tla.sh SyncHandshakeV1Fair` — 46,656\ndistinct states\n- `./scripts/verification/verify-tla.sh SyncHandshakeV1Mismatch` — 4,320\ndistinct states\n- `./scripts/verification/check-kani-coverage.sh` — 123/123 proofs\nregistered\n- focused verification-script tests — 153 passed\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- warmed A8 N=16 smoke/TCP release pair — 3.133 seconds combined;\npromotion deferred pending one green scheduled week\n- archived July 12 nightly shard 5 seed set — 125 seeds passed in\n134.699 seconds\n- focused repaired H-BLOAT release/hot-join nightly shape — 8.593\nseconds\n- `cargo nextest run --no-capture` — 2,870 passed, 74 skipped\n- `cargo test --test simulation` — 331 passed, 27 ignored\n- `cargo clippy --workspace --all-targets --features tokio,json,hot-join\n-- -D warnings`\n- `cargo doc --workspace --no-deps`\n- `python3 scripts/docs/check-wiki-consistency.py`\n- `python3 scripts/docs/check-links.py` — 1,438 links, zero\nerrors/warnings\n- focused H-OSC policy matrix, aggregation census, artifact bound,\nreplay, and shrink tests\n- focused H-META-RB 2x2 fixed/CPU × clean/spike experiment, {4,8,12} ms\nsensitivity, exact actuator/replay/artifact/shrink regressions\n- focused H-RING receipt/range, fail-safe census, artifact\nbounds/validation, replay-mutation, and shrink regressions\n- focused B2/B4 matched control/±12 census, artifact-v7\nround-trip/tamper validation, replay identity, and shrink\ntruncation/remapping regressions\n- `python3 -m pytest scripts/tests/test_check_benchmark_regressions.py\n-q` — 15 passed\n- `actionlint .github/workflows/ci-benchmarks.yml`\n- eight adversarial reviews (schema/artifact, experimental validity,\nwhole diff, hostile-gossip science/replay, dishonest-peer operator\npolicy, nightly H-BLOAT census, false-green protocol verification,\nhandshake trace-contract soundness/fail-closed behavior), all with zero\nactionable findings at their committed states\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> CI benchmark workflow changes affect merge gates; production session\nlogic is mostly observability and docs, but benchmark jobs are longer\nand more complex on PRs.\n> \n> **Overview**\n> **CI benchmarks** no longer fail PRs on cross-runner historical\ncomparisons. Pull requests run three paired base/merge Criterion cycles\non one runner (alternating order) and gate on median ratios with a 1.50\nthreshold and two-of-three votes via `check-benchmark-regressions.py`.\nHistorical gh-pages tracking moves to a separate non-PR job that never\nfails the build.\n> \n> **Telemetry** exposes `PeerMetrics::average_frame_advantage` as the\nsame rolling gauge `P2PSession` max-aggregates for wait recommendations.\n> \n> **Formal verification** drops the unused `NetworkProtocol.tla` module\nand disconnected protocol-state Kani transition proofs; docs narrow\nclaims to bounded `SyncHandshakeV1` models, enum checks, and Rust tests.\nA new **SyncHandshakeV1** NDJSON trace contract (`SyncHandshakeV1Trace`,\n`verify-sync-handshake-traces.py`) accepts matching/mismatch/timeout\nscenarios and rejects a single derived duplicate-reply mutation—without\nclaiming runtime refinement yet.\n> \n> **Operator docs** expand the threat model, desync playbook, and\nproduction checklist for equivocation, false checksum accusations,\nflooding, and hot-join snapshot poisoning (detection vs attribution).\n> \n> **Test hooks** add hidden `__internal` mutators and `P2PSession`\nhostile-gossip/receipt diagnostics for deterministic integration tests.\nSimulation nightly adds an H-OSC aggregation probe.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n718554f601407528387836a0a06b7e3c81f3f632. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-14T09:56:29-07:00",
          "tree_id": "05db7c77d1c87def502885dd8ce7bf0c8013b2d0",
          "url": "https://github.com/wallstop/fortress-rollback/commit/a3a1c6d15d6ec58c7f1b7dcb29ccaba37fcf7d5d"
        },
        "date": 1784048663331,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1000",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/4",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 30,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 38,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 84,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 282,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 55,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 136,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 451,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 25,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 106,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 131,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 173,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 183,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 255,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 378,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 346,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 474,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 719,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 183,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 209,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 258,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 352,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 419,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 551,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 650,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 780,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1043,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 486,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 657,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 878,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1084,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 114,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 167,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 455,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 701,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1024,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 2,
            "range": "± 0",
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
          "id": "6e3a641450bf85175eca7c58dd04b42cb5a49f0c",
          "message": "Add bounded handshake trace recorder (#239)\n\n## Summary\n\n- model handshake request identities as a fresh bounded namespace\nindependent of successful roundtrip count\n- strengthen the strict NDJSON trace contract with a genuine duplicated\nmessage and fail-closed schema validation\n- add an opt-in, fixed-capacity protocol-local handshake recorder that\ncompiles out of normal builds\n- classify overflow and raw request-ID collisions explicitly, including\ntimeout and hot-join rearm ordering\n\n## Validation\n\n- `cargo nextest run --no-capture` (2,869 passed)\n- `cargo nextest run --features hot-join --no-capture` (3,125 passed)\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `cargo clippy --workspace --all-targets --features\ntokio,json,trace-validation,hot-join`\n- `./scripts/verification/verify-tla.sh SyncHandshakeV1` (936,756\ndistinct states; all trace cases pass)\n- `python3 -m pytest -q\nscripts/tests/test_verify_sync_handshake_traces.py` (27 passed)\n- `python3 scripts/ci/agent-preflight.py`\n- rustdoc, links, Markdown, spelling, and allocation-bound checks\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Touches core sync handshake logic and formal specs; production\nbehavior is gated behind `trace-validation`, but spec/trace contract\nchanges affect CI verification breadth.\n> \n> **Overview**\n> Adds an unstable **`trace-validation`** Cargo feature and a\nfixed-capacity **`HandshakeTraceRecorder`** on `UdpProtocol` that\nrecords raw handshake transitions (sends, handlers, timeout,\nduplicate/collision dispositions) and fails closed on overflow or\nambiguous raw request-ID reuse; it is absent from default builds.\n> \n> **TLA+ / NDJSON contract:** `SyncHandshakeV1` now treats message\ntokens as a **fresh, bounded request-ID namespace** (`REQUEST_ID_COUNT`\n> `NUM_SYNC_PACKETS`) with monotonic `nextToken` (no wrap), a trace-only\n**`DuplicateMessage`** action, and **`TraceDelivery`** mode. The\nmatching NDJSON trace, Python verifier, and tests were updated for\ngenuine duplication, stricter integer `schema` validation, and the\nshifted reject mutation at step 9.\n> \n> Docs and design history note the recorder is landed while\nruntime-to-TLC normalization remains pending.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nee2c8a4ca2f2473760af39d7b5304cfc641e5fd6. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-14T11:37:00-07:00",
          "tree_id": "b94b3475c22e6166cfbf0f8c2d9cc283f366d031",
          "url": "https://github.com/wallstop/fortress-rollback/commit/6e3a641450bf85175eca7c58dd04b42cb5a49f0c"
        },
        "date": 1784054724402,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1000",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/4",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 36,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 78,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 299,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 126,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 476,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 94,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 119,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 155,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 159,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 214,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 314,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 290,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 403,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 604,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 146,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 170,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 207,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 263,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 318,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 431,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 544,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 633,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 846,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 522,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 624,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 785,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 944,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 93,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 127,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 554,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 932,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1416,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 2,
            "range": "± 0",
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
          "id": "94f33a86fc9b20bb59977e7247029b3163372ee6",
          "message": "Validate runtime handshake traces (#240)\n\n## Summary\n\n- add a hidden, semantically capped handshake trace recorder option and\nsession trace accessor under `trace-validation`\n- produce deterministic matching, mismatch, and timeout traces from two\nreal `P2PSession` peers over SimNet\n- normalize raw request IDs into the finite TLA+ domain and validate\nruntime-derived accept/reject cases with TLC\n- make trace generation a hard-fail verification boundary and wire its\nRust dependency into CI\n- document the runtime refinement boundary, remaining abstractions, and\ndesign rationale\n\n## Why\n\nThe existing SyncHandshakeV1 trace gate only replayed hand-authored\nfixtures. That proved the checker, but not that the Rust handshake\nimplementation's observable behavior refines the model. This change adds\nan ephemeral runtime producer and independently projects recorded\npost-state into strict NDJSON before TLC replay.\n\n## Impact\n\nThe runtime APIs are hidden and feature-gated. Production behavior is\nunchanged unless `trace-validation` is explicitly enabled, and recorder\ncapacity is bounded to 64 events.\n\n## Verification\n\n- `cargo fmt --all`\n- `cargo clippy --workspace --all-targets --features\ntokio,json,trace-validation,hot-join -- -D warnings`\n- full nextest suites: default (2869), hot-join (3125), trace-validation\n(2879)\n- `python3 -m pytest -q\nscripts/tests/test_verify_sync_handshake_traces.py` (29)\n- `python3 scripts/verification/verify-sync-handshake-traces.py` (8 TLC\ntrace cases)\n- `./scripts/verification/verify-tla.sh SyncHandshakeV1`\n- `cargo doc --no-deps --features trace-validation`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- `actionlint .github/workflows/ci-verification.yml`\n\n## Review readiness\n\n| Area | Evidence |\n| --- | --- |\n| Correctness | runtime matching/mismatch/timeout scenarios plus model\naccept/reject replay |\n| Determinism | TestClock, seeded SimNet, ordered maps/sets, monotonic\nID normalization |\n| Safety | bounded recorder, structured errors, no production panics |\n| CI portability | stable Rust installed in TLA job; producer failures\nand incomplete manifests hard-fail |\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Touches the handshake verification boundary and adds hidden\nsession/protocol hooks, but they are feature-gated and default builds\nstay unchanged; CI TLA runs now depend on a successful Rust trace\nproducer.\n> \n> **Overview**\n> Extends the **SyncHandshakeV1** NDJSON trace gate so CI no longer\nrelies only on hand-authored fixtures: on the default manifest it now\nruns a feature-gated **`cargo test`** producer, checks a four-file\nephemeral runtime manifest, normalizes events, and runs **TLC** on eight\ncases (four static + four runtime).\n> \n> **Rust (behind `trace-validation`):** hidden\n**`SessionBuilder::with_handshake_trace_capacity`** (1–64 records per\nendpoint) and **`P2PSession::handshake_trace`** expose the existing\nbounded recorder. **`tests/simulation/trace_validation.rs`** drives\ntwo-peer **`P2PSession`** over **SimNet** (matching with duplicate reply\n/ delayed B, mismatch, timeout), merges events in deterministic poll\norder, maps raw request IDs to trace-local ordinals, writes NDJSON when\n**`FORTRESS_RUNTIME_TRACE_DIR`** is set, and derives the negative\nmutation from the observed duplicate row.\n> \n> **Python:** **`generate_runtime_traces`** /\n**`validate_runtime_manifest`** hard-fail on producer exit, timeout, or\nincomplete output; unit tests cover those paths.\n> \n> **CI:** the TLA verification job installs **stable Rust** and caches\nbuilds for the producer; workflow path filters include the trace script.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n61ab4f9314aaebf0818eb1ef666b7d452781517c. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-14T13:35:27-07:00",
          "tree_id": "6e6247f42acb492dc48d0c806d84e9d481493ec4",
          "url": "https://github.com/wallstop/fortress-rollback/commit/94f33a86fc9b20bb59977e7247029b3163372ee6"
        },
        "date": 1784061856221,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1000",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/4",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 37,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 278,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 55,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 135,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 450,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 25,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 106,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 129,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 167,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 181,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 253,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 363,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 345,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 474,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 688,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 182,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 207,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 254,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 350,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 416,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 541,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 650,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 781,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1027,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 493,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 627,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 848,
            "range": "± 260",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1055,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 115,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 164,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 430,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 683,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1017,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 2,
            "range": "± 0",
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
          "id": "1934c2b3d5e8843cb00815e0d8d6bd176274ff2c",
          "message": "Add browser transport throughput diagnostics (#243)\n\n## Summary\n\n- record per-player protocol-message enqueue demand in the deterministic\nsweep and schema-v3 baseline\n- distinguish Fortress enqueue demand from socket-adapter, relay, and\nobserved network throughput across metrics and public documentation\n- document the bounded multi-message burst/backpressure contract for\nasynchronous `NonBlockingSocket` adapters\n\n## Root cause\n\nThis is the Fortress-side diagnostic and contract work for #242. Signal\nFish client v0.8.0's Godot WebSocket transport submits one frame, then\nreports it pending until the socket-wide browser\n`WebSocket.bufferedAmount` returns to zero. The polling client stops at\nthat first pending send. Browser event-loop semantics therefore limit\nthis path to one new WebSocket frame per rendered callback.\n\nThe deterministic clean two-player sweep measures 135.8125 protocol\nmessages enqueued per player per second, including one-time\nsynchronization traffic (135.1875 steady state). A 16 ms polling\ncallback can service at most 62.5 messages per second under the upstream\nstop-and-wait behavior. Accounting for time-based control traffic gives\nthe approximate capacity model `2F + F/30 + 10 <= 62.5`, or `F <= 25.8`,\nbefore network, relay, or browser costs.\n\nFortress's default eight-frame prediction stall is a safety throttle\nafter confirmations fall behind, not the root cause. Server reliable\nFIFO delivery and batching can amplify latency but are not necessary for\nthe capacity mismatch.\n\nOwning upstream issue:\nhttps://github.com/Ambiguous-Interactive/signal-fish-client-rust/issues/61\nServer residual-risk discussion:\nhttps://github.com/Ambiguous-Interactive/signal-fish-server/issues/136#issuecomment-4987500021\nFull Fortress RCA:\nhttps://github.com/wallstop/fortress-rollback/issues/242#issuecomment-4987500310\n\n## Impact\n\nThis PR intentionally does not change Fortress production pacing,\nacknowledgement, or prediction behavior. It adds durable, data-driven\nevidence for offered protocol demand and prevents custom transport\nintegrations from misreading enqueue counters as accepted/physical\nthroughput.\n\nThe upstream browser transport fix and a real Godot/browser/server E2E\nremain required before #242 can close.\n\n## Validation\n\n- `cargo check`\n- `cargo nextest run --no-capture`: 2,869 passed, 74 skipped\n- `cargo nextest run --features hot-join --no-capture`: 3,125 passed, 75\nskipped\n- `cargo test --test simulation sweep_pr_gate -- --nocapture`\n- `cargo test --test simulation\npeer_wire_metrics_are_wired_across_smoke_fleet -- --nocapture`\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `cargo doc --no-deps`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- markdown, links, wiki consistency, semantic documentation, spelling,\nformatting, and diff checks\n\n## Review Readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS\n- Determinism: PASS\n- Agent preflight: PASS\n- Error handling: PASS\n- Tests breadth: PASS\n- Design log reviewed: N/A — no production architecture change\n- CHANGELOG reviewed: N/A — diagnostics/contracts only; no released\nbehavior change\n\nRefs #242.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Documentation, rustdoc, and deterministic test baseline only; runtime\nprotocol and session logic are unchanged per the PR scope.\n> \n> **Overview**\n> Clarifies that **`PeerMetrics`**, **`NetworkStats::kbps_sent`**, and\nrelated APIs measure **Fortress protocol enqueue demand** (encoded\nbytes/messages entering the socket-bound queue), not adapter acceptance\nor observed network throughput. Public rustdoc, API contracts,\ntelemetry, tuning, production checklist, and user-guide text are updated\naccordingly, including guidance to compare\n**`PeerMetrics::packets_sent`** deltas with custom transport admission\nrate, queue depth, and oldest-message age.\n> \n> Documents an explicit **`NonBlockingSocket`** contract: one session\nupdate may call **`send_to`** multiple times; async adapters must return\npromptly with bounded bursts or freshness-preserving drop policy, and\nmust not block until the outbound buffer empties after each message\n(stop-and-wait).\n> \n> The deterministic baseline sweep moves to **schema v3**\n(`sweep-v3.json`), adding\n**`protocol_messages_enqueued_per_player_per_sec`** to **`CellReport`**,\nPR gate comparisons, and the checked-in ledger; simulation harness\ncomments and fleet tests use the same “protocol-cost / enqueue demand”\nvocabulary. **No production pacing, acknowledgement, or prediction\nbehavior changes.**\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n9fb91d1e929b71312ae16162844f26d73e4cc1b8. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-15T21:15:04-07:00",
          "tree_id": "2c4fa61d66cd53af3bd6a1944aa09165749cc2c6",
          "url": "https://github.com/wallstop/fortress-rollback/commit/1934c2b3d5e8843cb00815e0d8d6bd176274ff2c"
        },
        "date": 1784175808086,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1000",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/4",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 279,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 35,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 56,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 136,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 452,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 105,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 131,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 169,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 181,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 255,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 375,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 348,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 477,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 706,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 182,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 208,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 256,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 353,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 418,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 547,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 653,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 780,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1046,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 483,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 619,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 844,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1043,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 115,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 164,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 431,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 694,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1025,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 2,
            "range": "± 0",
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
          "id": "bc8bdc792bc4d7bcb9f7f0a0afa0a30578bd3303",
          "message": "Harden simulation coverage, cost evidence, and guidance (#241)\n\n- add deterministic Swarm simulation coverage with materialized replay\n- document tested topology, pacing semantics, relay limits, and operational misuse guidance\n- add the isolated H-16P confirmation-fold benchmark and informational CI tracking",
          "timestamp": "2026-07-15T21:59:13-07:00",
          "tree_id": "c204cc3f833af6399f8fda189fb0fc0723fa555c",
          "url": "https://github.com/wallstop/fortress-rollback/commit/bc8bdc792bc4d7bcb9f7f0a0afa0a30578bd3303"
        },
        "date": 1784178496404,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1000",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/4",
            "value": 28,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 279,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 60,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 140,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 451,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 25,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 104,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 132,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 172,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 181,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 259,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 359,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 345,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 480,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 688,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 181,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 209,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 256,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 352,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 418,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 549,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 651,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 773,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1022,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 493,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 623,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 829,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1034,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 114,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 165,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 431,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 701,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1036,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 18,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 83,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 314,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1507,
            "range": "± 2",
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
          "id": "42935ad490c85e49f3654aa66a02ce49cd0cca70",
          "message": "Migrate project guidance to Agent Skills (#246)\n\n## Summary\n\n- migrate the canonical development guide and 48 focused guides from\n`.llm/` into discoverable open-format skills under `.agents/skills/`\n- move the question template and design history into skill-local\n`assets/` and `references/`\n- replace the bespoke index/line-limit tooling with fail-closed YAML\nvalidation, code-example checks, pre-commit integration, and dedicated\nCI regression coverage\n- update every live repository reference, packaging exclusion, workflow,\nhook, and test for the new layout\n- make agent preflight robust to deleted workflow, Python, and Rust\npaths found during adversarial review\n\n## Why\n\nThe legacy `.llm/` hierarchy required custom discovery and index\nmaintenance and was not directly discoverable by Agent Skills-compatible\ntools. The new layout uses the portable `SKILL.md` contract while\npreserving the repository's complete policy and specialist guidance.\n\n## Validation\n\n- 49 skills and 51 skill Markdown resources validated\n- 1,670 script tests passed\n- 2,874 default Rust tests passed; 74 skipped\n- 3,130 hot-join Rust tests passed; 75 skipped\n- strict all-target Clippy with `tokio,json` passed\n- warning-denied workspace docs and formatting passed\n- agent preflight, actionlint, YAML, markdownlint, links, wiki\nconsistency, shell portability, typos, package contents, and hook-output\nchecks passed\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Large documentation and CI migration with broad reference updates;\nincorrect links or preflight gaps could misroute agents or miss\nvalidation until CI runs, but no production Rust networking logic\nchanges.\n> \n> **Overview**\n> This PR **replaces the `.llm/` tree** with **49 discoverable skills**\nunder `.agents/skills/`, each as `SKILL.md` with YAML frontmatter\n(`name`, `description`, etc.). The former canonical `context.md` becomes\n**`fortress-development`**; workflow guides link sibling skills via\n`../other-skill/SKILL.md` instead of category paths. **Design history**\nmoves into `design-decisions/references/`; the **ask-user template**\ninto `fortress-development/assets/`.\n> \n> **Tooling and gates change:** `check-llm-line-limit`,\n`regenerate-skills-index`, and `ci-llm-lint.yml` are **removed** in\nfavor of `validate-agent-skills.py`, `check-agent-skills.sh` (500-line\ncap on skill markdown), pre-commit hooks, and **`ci-agent-skills.yml`**.\nThe validator **fails if `.llm/` still exists** and enforces open-format\nrules (directory name match, required fields, duplicate YAML keys).\n> \n> **References and packaging** now point agents and humans at\n`.agents/skills/fortress-development/SKILL.md` (`AGENTS.md`,\n`CLAUDE.md`, `.cursorrules`, Copilot, `llms.txt`, changelog internal\npatterns, `Cargo.toml`/`.dockerignore` excludes). **Agent preflight**\nruns agent-skill checks instead of LLM line/index checks, lints **all\nworkflows** when any workflow changes (not only changed paths), and\n**skips deleted paths** when passing file lists to scanners.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n755ee4fc5c22c4d95c3a4a81cb620af51593e03d. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-16T10:30:37-07:00",
          "tree_id": "7a1bf63087abb9c09ccdb290271ca74c53470e8b",
          "url": "https://github.com/wallstop/fortress-rollback/commit/42935ad490c85e49f3654aa66a02ce49cd0cca70"
        },
        "date": 1784223606919,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1000",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/4",
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 83,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 280,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 60,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 155,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 525,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 27,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 105,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 133,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 177,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 182,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 262,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 382,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 350,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 481,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 721,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 182,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 212,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 259,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 353,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 426,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 557,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 651,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 784,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1061,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 482,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 619,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 842,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1057,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 115,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 166,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 430,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 688,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1013,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 3,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 78,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 292,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1366,
            "range": "± 6",
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
          "id": "f19f384aac0547eb63eb991e9510da7301c175f0",
          "message": "Document D12 frame-domain boundary (#247)\n\n## Summary\n\n- document the exact bounded-wire domain for D12 frame fields\n- pin `i32::MAX` compatibility across connection status, floor replies,\nand checksum reports\n- retain the existing rejection of invalid negative frames without\nintroducing a narrower protocol cap\n\n## Why\n\n`Frame` deliberately supports the complete non-negative `i32` range. D12\nhad already closed the negative-domain validation gap, but its\nupper-bound disposition remained implicit. An arbitrary smaller cap\nwould reject values supported by the public type and change protocol\ncompatibility.\n\n## Impact\n\nThis is a test and documentation clarification only. It changes no wire\nbytes, production branches, allocation bounds, public API, or\ndeterministic behavior.\n\n## Validation\n\n- negative-control mutation proved the new regression is load-bearing\n- `cargo nextest run --no-capture` — 2,875 passed\n- `cargo nextest run --features hot-join --no-capture` — 3,131 passed\n- `cargo test --doc -- --nocapture` — 160 passed\n- Clippy, rustdoc, rustfmt, markdownlint, and `git diff --check`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- main-thread adversarial review found no high- or critical-severity\nissue\n\n## Changelog\n\nNo changelog entry: no public or user-observable behavior changes.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Documentation and test-only changes with no modifications to decode\nlogic or wire compatibility.\n> \n> **Overview**\n> Clarifies the **D12** bounded-wire contract for frame fields: decoders\naccept the full public [`Frame`] domain (non-negative values through\n`i32::MAX`, plus [`Frame::NULL`] only where semantics allow), with\n**no** narrower protocol cap in `read_frame`.\n> \n> Docs on `read_frame` and on `ConnectionStatus::last_frame`,\n`ChecksumReport::frame`, and `FloorReply::floors` spell out that rule\nand note that checksum frames reject the null sentinel.\n> \n> Adds **`decode_message_accepts_maximum_frame_for_all_d12_fields`**,\nwhich round-trips `i32::MAX` through Input connect status, floor\nreplies, and checksum reports via `decode_message`. **No wire format,\ndecode branches, or public API behavior change**—only documentation and\na regression test.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n576855c235f49d59851fe0232268d51a2c368165. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-16T12:33:09-07:00",
          "tree_id": "54ef2ed57ca64f5e186a414062d14298f5cc0511",
          "url": "https://github.com/wallstop/fortress-rollback/commit/f19f384aac0547eb63eb991e9510da7301c175f0"
        },
        "date": 1784230912810,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1000",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/4",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 38,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 93,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 307,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 60,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 161,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 581,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 98,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 123,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 169,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 163,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 229,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 348,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 295,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 420,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 673,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 158,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 185,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 233,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 290,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 350,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 476,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 545,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 662,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 943,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 502,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 620,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 835,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1030,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 132,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 178,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 489,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 770,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1138,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 311,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1236,
            "range": "± 4",
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
          "id": "ce340e676f329009177dd487118c34cebc23dc51",
          "message": "Harden release workspace lock synchronization (#252)\n\n## Summary\n\nPermanently repairs the release-lock synchronization failure behind PR\n#251 by making Cargo authoritative for every tracked workspace root.\n\n- dynamically discovers root, `fuzz`, Loom, Godot, and future standalone\nworkspaces\n- rejects missing, malformed, orphan, and member-local locks\n- synchronizes locks with `cargo update --workspace` and validates them\nwith full `cargo metadata --locked --all-features`\n- makes release preparation sandboxed, rollback-capable, topology-aware,\nand dry-run immutable\n- enforces the invariant in prepare, version-sync, publish, Loom CI,\nhooks, agent preflight, and maintainer guidance\n- removes obsolete `tests/network-peer/Cargo.lock`, because that member\nshares the root lock\n\n## Red evidence\n\n- The structural checker rejected the obsolete network-peer member lock.\n- A realistic root-version bump with only the root lock updated leaves\nall three standalone locks stale and fails.\n- A dependency-only stale fixture passes structural inspection but fails\nfull locked metadata, guarding against the former vacuous `--no-deps`\noracle.\n- Post-merge recovery simulation proved dry-run omitted the updated\n`[Unreleased]` comparison link and minor/major version-reference changes\nwhile `sync-version.sh` ran outside the sandbox.\n- Cursor’s force-tracked ignored-path fixture proved ordinary sandbox\nindexing could silently shrink the tracked set.\n- Failure injection proved rollback must recreate a lock deleted by a\nfailed Cargo subprocess.\n- Adversarial review found and fixed concurrent-output overwrite and\nmissing-tracked-input topology shrinkage.\n\n## Green verification\n\n- `python3 -m pytest -q scripts/tests`: **1,703 passed**\n- focused release/hook/workflow/preflight tests: **115 passed**\n- canonical checker: all four workspace roots passed\n- real patch dry run: byte-for-byte immutable; all four lock diffs and\nthe updated `[Unreleased]` link emitted\n- non-dry release simulation: full checks passed and both canonical\nsynchronizers were idempotent\n- Loom exact gate: **19 passed** with `--release --locked`\n- Godot pinned-nightly `clippy --locked --all-targets --all-features`:\npassed\n- `cargo fmt --all -- --check`: passed\n- workspace Clippy with `tokio,json`: passed\n- `cargo nextest run --no-capture`: **2,875 passed, 74 skipped**\n- warning-free workspace docs, actionlint, Agent Skill validators, shell\nportability, Markdown, links, doc claims, and agent preflight: passed\n\n## Review readiness\n\n- [x] No Rust public API or runtime behavior change\n- [x] No changelog entry required (internal release/CI tooling)\n- [x] Main-thread adversarial review completed; no high/critical finding\nremains\n- [x] Progress record:\n`progress/session-144-release-lock-synchronization.md`\n- [x] Hardening is isolated from the later minimal v0.10.1 repair to PR\n#251\n\nThis intentionally does not cherry-pick closed PR #237, whose validator\nretained the proven-vacuous `--no-deps` behavior.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Changes release preparation, publication gates, and lock validation\nacross CI—mistakes could block releases or pass stale locks, but scope\nis tooling-only with heavy regression coverage and no production Rust\nchanges.\n> \n> **Overview**\n> Hardens release and CI tooling so **every tracked Cargo workspace\nroot** gets an authoritative `Cargo.lock`, replacing root-only textual\nlock edits and the vacuous `cargo metadata --locked --no-deps` check.\n> \n> **New `scripts/release/workspace_locks.py`** dynamically discovers\nworkspace roots via `cargo locate-project`, rejects orphan/member-local\nlocks, syncs with `cargo update --workspace`, and validates with full\n`cargo metadata --locked --all-features`. **`prepare_release.py`** now\nruns in a tracked-file Git sandbox: bumps manifest/changelog, runs lock\nsync and `sync-version.sh` inside the transaction, validates topology,\nand applies outputs atomically with rollback; dry-runs leave the live\ntree unchanged.\n> \n> **Workflow and gate wiring:** `release-prepare.yml` runs release-tool\ntests before mutation, proves canonical sync is idempotent, and drops\n`--no-deps`; `publish.yml` and `ci-version-sync.yml` run the full lock\nchecker; Loom CI uses `cargo test --release --locked`.\n**Hooks/preflight:** pre-commit `check-structure` on Cargo/release\nsurfaces; agent preflight adds `workspace-lock-check`. **Docs/skills**\ndocument the workspace lock rule and reviewed release path. **Removes**\nobsolete `tests/network-peer/Cargo.lock` and updates the network-peer\nmanifest comment to reflect root lock sharing.\n> \n> Extensive regressions in `test_workspace_locks.py`,\n`test_prepare_release.py`, and workflow contract tests; no Rust public\nAPI or runtime behavior changes.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n60228694644309bccb64cfb14292b3393c466143. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-17T18:47:13-07:00",
          "tree_id": "4fc4caf288934819c0735939d9da6f1bbdc3ca5d",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ce340e676f329009177dd487118c34cebc23dc51"
        },
        "date": 1784339746461,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1000",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/4",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 38,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 93,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 307,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 60,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 161,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 580,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 45,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 101,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 126,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 168,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 162,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 229,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 335,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 296,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 420,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 672,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 157,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 184,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 232,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 289,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 350,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 476,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 549,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 670,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 944,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 458,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 624,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 857,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1056,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 121,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 167,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 458,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 759,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1109,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 311,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1233,
            "range": "± 5",
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
          "id": "9ae76e1cbce0a47e8bd1da734c4bbefdb1ced4b3",
          "message": "Harden release and publish automation (#254)\n\n## Summary\n\nHardens the complete release path after PR #253 and Actions job\n88030023204 exposed two independent failure classes:\nrelease-state-dependent tests after preparation and floating Rust\nchannel manifest races.\n\n- derives the minimum SemVer bump from curated changelog categories (the\ncurrent release is correctly planned as 0.11.0, not 0.10.1);\n- reconstructs generated release PRs from trusted base code and compares\nthe exact tracked tree;\n- makes preparation reruns/stale branches recoverable with exact leases\nand an atomic main/release-branch CAS;\n- records a reviewed source manifest and resolves first publication from\nthe unique valid prepared commit, even after main advances or the prior\ntag is older than 256 commits;\n- creates/revalidates exact annotated-tag checkpoints before crates.io\nand GitHub mutations;\n- reconciles ambiguous Cargo failures against the crates.io checksum for\nidempotent retries;\n- pins stable, nightly, Miri, Python, actions, and hash-locked Python\ntest dependencies;\n- runs trusted release-state checks on every PR and on merge-group\nprospective trees;\n- adds executable regressions, agent preflight coverage, LLM/skill\npolicy, and an architectural decision trail.\n\n## Evidence\n\n- 1,955 complete Python/script tests pass.\n- Agent preflight passes: 275 release tests, 66 toolchain contracts, 49\nskills, actionlint, changelog, 5,138-file/1,392-link validation,\nfallback-import and spelling gates.\n- Full Rust fmt, workspace Clippy with `-D warnings`, and\nworkspace/all-targets tests and benchmarks pass with `tokio,json`.\n- A `--bump minor` dry run deterministically produces 0.10.0 → 0.11.0\nacross all locks/docs/wiki.\n- Cursor Bugbot reviewed exact final commit `596a162` with no new\nissues; all four earlier actionable threads are fixed and resolved.\nCopilot was requested after every push but reports an account quota\nlimit.\n\n## Required repository rollout\n\nBefore merging a generated `release/v*` PR, an administrator must\nrequire the stable **Verify prepared release state** check on `main` and\nenable merge queue (preferred) or strict “require branches to be up to\ndate” checks. GitHub owns this repository setting; the workflow now\nsupplies both PR and `merge_group` checks but repository code cannot\nactivate the protection itself.\n\n## Follow-up\n\nOnce this hardening PR is merged, close/supersede #253 and run **Release\n- Prepare PR** with a minor bump to generate the reviewed v0.11.0\nrelease PR.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **High Risk**\n> Changes irreversible release and publish automation, annotated-tag\ntrust boundaries, and semver classification; misconfiguration could\nblock releases or allow publishing the wrong tree without the required\n**Verify prepared release state** branch protection.\n> \n> **Overview**\n> Hardens the full **prepare → review → publish** path so the merged\ntree is the only source of truth for crates.io and GitHub releases, with\nstricter semver and reproducible CI toolchains.\n> \n> **Release policy and changelog:** Preparation now derives a **minimum\nSemVer bump** from `[Unreleased]` categories (`release_policy.py`,\nenforced in `prepare_release.py` and agent skills). Wire-protocol v2 is\ndocumented under `### Changed` with `**Breaking:**` instead of a `Fixed`\nentry. Release dates and issue-template versions are finalized in the\npreparation PR; post-publish default-branch metadata commits are\nremoved.\n> \n> **Immutable prepared state:** New `release-state.json` / digest\nverification, `ci-release-state.yml` (trusted base + candidate checkout\non PRs and merge groups), branch recovery (`release_branch.py`), and\npublish-time candidate resolution (`release_checkpoint.py`) with\nannotated-tag checkpoints revalidated before registry and GitHub\nmutations. `publish_state.py` reconciles crates.io by checksum for\nidempotent retries. `publish.yml` no longer auto-fixes changelog or\npushes tags from `main` alone.\n> \n> **Tooling pins:** Composite actions install **dated nightly**,\nseparate Miri nightly, and **pinned stable** release Rust with bounded\nretries; required workflows switch off floating\n`dtolnay/rust-toolchain@nightly`. Release workflows use Python 3.13.5\nand hash-locked `requirements.txt`.\n> \n> **Docs and preflight:** Publishing/changelog/fortress-development\nskills and `agent-preflight.py` expand release and toolchain contract\ntests.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n596a162c12f24dbcb2233dbc6efac6e1cb5591fd. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-18T09:20:04-07:00",
          "tree_id": "d8bab669f11681637798bdf78062ee70620e695b",
          "url": "https://github.com/wallstop/fortress-rollback/commit/9ae76e1cbce0a47e8bd1da734c4bbefdb1ced4b3"
        },
        "date": 1784392099639,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1000",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/4",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 38,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 93,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 307,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 36,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 60,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 161,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 582,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 32,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 98,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 126,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 169,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 162,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 228,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 346,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 295,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 419,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 673,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 157,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 184,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 233,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 290,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 350,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 475,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 540,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 674,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 937,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 457,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 617,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 851,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1044,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 121,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 169,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 464,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 734,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1106,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 2,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 85,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 314,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1233,
            "range": "± 5",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}