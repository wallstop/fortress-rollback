window.BENCHMARK_DATA = {
  "lastUpdate": 1783958611735,
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
      }
    ]
  }
}