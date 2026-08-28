window.BENCHMARK_DATA = {
  "lastUpdate": 1787899209054,
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
      },
      {
        "commit": {
          "author": {
            "email": "304587157+wallstop-auto-releaser[bot]@users.noreply.github.com",
            "name": "wallstop-auto-releaser[bot]",
            "username": "wallstop-auto-releaser[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "082c53e9170a631795e113864570e59370a935ba",
          "message": "Prepare v0.11.0 release (#257)\n\nPrepare the reviewed v0.11.0 release state.",
          "timestamp": "2026-07-18T11:11:59-07:00",
          "tree_id": "158676f5a48ef61271beec16e34da0ea5690a275",
          "url": "https://github.com/wallstop/fortress-rollback/commit/082c53e9170a631795e113864570e59370a935ba"
        },
        "date": 1784398806373,
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
            "value": 308,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 581,
            "range": "± 1",
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 125,
            "range": "± 0",
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
            "value": 163,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 222,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 346,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 294,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 419,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 671,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 159,
            "range": "± 9",
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
            "value": 232,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 289,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 345,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 475,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 544,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 665,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 940,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 464,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 627,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 823,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1042,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 120,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 168,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 465,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 745,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1078,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 21,
            "range": "± 4",
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
            "value": 1229,
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
          "id": "a9908b85000029e8116c60abd0df38cc7e62efad",
          "message": "Harden scheduled simulation fleet and CI pin contracts (#262)\n\n## Summary\n\n- plant Reliable FIFO HOL windows before generated lifecycle faults so\nthe sender remains active\n- serialize and propagate a test-only disconnect-timeout override\nthrough peers, spectators, hot-join replacements, artifacts, and\nshrinking\n- keep non-clean nightly transport rows free of implicit terminal\nmembership changes while retaining production timeouts for clean\nlifecycle coverage\n- pin both July 2026 scheduled CI failure seeds as an ignored long\nregression\n- synchronize workflow contract tests and version comments with the\naction pins already merged by #261\n\nThis repairs the two failures that reset A8's scheduled-history gate; it\ndoes not promote A8 before seven consecutive scheduled `main` runs\nexist. The CI-pin follow-up repairs a pre-existing `main` mismatch\nexposed by this PR's Documentation run and does not change workflow\nbehavior.\n\n## Incident evidence\n\n- scheduled run 29718845598, seed `29718845598175`: the planted peer-7\nHOL link overlapped a peer-7 stall, so no retransmission could occur\n- scheduled run 30068604101, seed `30068604101890`: Rough loss caused an\nimplicit timeout and expected fail-closed membership split inside the\ntransport-only green tier\n- both exact seeds now pass the full oracle; the Reliable FIFO seed\nrecords positive `retransmit_delayed` evidence\n- Documentation run 30186497038: three assertions still expected the\npre-#261 `setup-python` and `rust-toolchain` pins; the complete contract\naudit also found and corrected the stale `setup-node@v6` assertion\n\n## Review readiness\n\n- Build/tests: PASS — 2,878 default tests and 222 doctests\n- Script tests: PASS — all 1,968 tests\n- Strict Clippy: PASS — default and `hot-join` configurations, warnings\ndenied\n- Exact historical seeds: PASS — ignored focused regression, 31.96s\ndebug\n- Workflow lint: PASS — actionlint on all workflows\n- Agent preflight: PASS — all checks, including release automation and\nCI toolchain contracts\n- Zero-panic/determinism: PASS — test-only simulation change; no\nproduction paths modified\n- Error handling: PASS — zero timeout rejected and covered\n- Tests breadth: PASS — premise, clean/non-clean split, serde, shrinker,\nhot-join propagation, exact seeds, exact action pins\n- Design log: N/A — no production architecture or public behavior change\n- CHANGELOG: N/A — test/CI contract repair only\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes are confined to simulation tests/harness and CI pin\nassertions; production session APIs are only invoked via existing\nbuilder hooks in tests.\n> \n> **Overview**\n> **Simulation fleet** fixes two scheduled CI failures by reshaping\n**Reliable FIFO** head-of-line (HOL) injection and how long nightly\ntransport runs wait before disconnect.\n> \n> HOL windows are planted **before step 100** (lifecycle faults cannot\nstart earlier), so the chosen sender stays active instead of overlapping\nstalls/kills. A test-only **`RunOptions::disconnect_timeout`** is\nthreaded through session, spectator, and hot-join builders (plus shrink\nremapping) so **non-clean** nightly rows use a timeout longer than the\nmodeled run—avoiding implicit membership splits from random loss while\n**clean** rows keep default options.\n> \n> New unit tests lock the HOL premise, timeout behavior, serde/shrink\npropagation, and an ignored long regression over the two July 2026\nfailure seeds.\n> \n> **CI contracts** align workflow YAML and Python contract tests with\npins already on `main`: `actions/setup-python` v7 SHA,\n`dtolnay/rust-toolchain` and `actions/setup-node@v7` expectations—no\nintended workflow behavior change beyond those pin comments/versions.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nfa68bef60420a418273e085c5c8e3176a617e7fb. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-26T09:09:19-07:00",
          "tree_id": "d76e77b6ccaf36437dbe3fb59866d92bb84aebcb",
          "url": "https://github.com/wallstop/fortress-rollback/commit/a9908b85000029e8116c60abd0df38cc7e62efad"
        },
        "date": 1785082681958,
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
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 38,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 91,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 335,
            "range": "± 12",
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
            "value": 155,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 568,
            "range": "± 11",
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
            "value": 25,
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
            "value": 102,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 124,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 159,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 175,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 220,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 320,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 320,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 412,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 595,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 163,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 186,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 222,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 295,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 346,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 439,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 570,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 690,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 885,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 545,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 770,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 960,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1091,
            "range": "± 13",
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
            "value": 129,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 564,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 955,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1474,
            "range": "± 18",
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
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 286,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1093,
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
          "id": "88b6c3040482e05118caf56279b638dbd8ad3982",
          "message": "Promote N=16 controls and refresh dependency CI (#267)\n\n## What\n\nPromotes the four coordinated A8 N=16 simulation controls into the\nnormal PR test suite:\n\n- smoke-fleet invariants\n- reliable-FIFO/TCP invariants\n- seeded-divergence oracle coverage\n- bit-identical trace determinism\n\nThe N=12 smoke probe and 5,000-step N=16 budget diagnostic remain manual\nand ignored.\n\nThis PR also closes the dependency-maintenance work exposed by the\npromotion:\n\n- aligns `wasm-bindgen-cli` with locked `wasm-bindgen 0.2.126`\n- preserves Rust 1.86 compatibility with `serial_test 3.5.0` and a\nDependabot semver-major guard\n- refreshes all 65 compatible Cargo packages (`cargo update --dry-run`\nnow locks 0 packages)\n- pins `taiki-e/install-action@v2.85.6` and `docker/login-action@v4.6.0`\n- gives the promoted smoke control an evidence-backed 180-second local\nNextest ceiling, with retries still disabled\n- gives tarpaulin a 300-second response timeout inside the existing\n30-minute job cap\n\n## Why\n\nA8 required both an approximately 10-second PR budget and seven\nconsecutive scheduled `main` successes. The release-mode promotion set\nruns in 4.250 seconds locally, and the scheduled simulation fleet\ncompleted eight uninterrupted successful runs from 2026-07-25 through\n2026-08-01.\n\nThe initial PR runs then exposed three pre-existing CI incompatibilities\nfrom the preceding dependency update: a wasm-bindgen schema mismatch, an\nMSRV-incompatible serial_test major, and tarpaulin's 60-second\ninstrumented-response default. Each was reproduced before repair.\n\n## Impact\n\nNo production, public API, wire-format, allocation, or session-runtime\nbehavior changes. Changes are limited to test promotion, CI/tool\nconfiguration, dependency resolution, and regression contracts. No\nchangelog entry is required.\n\n## Review readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS (no production diff)\n- Determinism: PASS\n- Agent preflight: PASS\n- MSRV and supply-chain checks: PASS\n- Error handling: N/A\n- Changelog: N/A\n\n## Validation\n\n- release-mode promotion set: 4/4 passed in 4.250 seconds\n- default Nextest: 2,882/2,882 passed\n- hot-join Nextest: 3,138/3,138 passed\n- focused loaded N=16 smoke: passed in 60.624 seconds on its first\nattempt under the new 180-second ceiling\n- `cargo +1.86 check --all-targets`\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`\n- `cargo deny check`\n- `cargo fmt --check`\n- warning-denied docs with `hot-join,tokio,json`\n- 1,969/1,969 script tests\n- actionlint and agent preflight, including 66 CI toolchain contract\ntests\n- real wasm browser smoke with CLI 0.2.126: 1/1 passed\n- exact tarpaulin command: 53.54% line coverage (7,034/13,139)\n- two-round main-thread adversarial review; repeat sweep clean\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> No production code changes, but PR CI now depends on longer simulation\nruns and a large lockfile refresh; serial_test 3.x and refreshed\ntransitive deps could affect test ordering/isolation if misused.\n> \n> **Overview**\n> Promotes **four A8 N=16 simulation controls** from ignored/manual runs\ninto the normal PR suite: smoke-fleet invariants, TCP reliable-FIFO\ninvariants, seeded oracle divergence, and bit-identical trace\ndeterminism. The N=12 smoke probe and other large-mesh diagnostics stay\n**`#[ignore]`**.\n> \n> **CI and tooling** are adjusted so those tests and the dependency\nrefresh stay green: a **180s** Nextest slow-timeout override for the\npromoted smoke probe, **300s** tarpaulin `--timeout` for instrumented\nN=16 work, **`wasm-bindgen-cli` pinned to locked 0.2.126** (with\n**`taiki-e/install-action@v2.85.6`**), **`serial_test` held at 3.5** for\nRust **1.86** MSRV plus a Dependabot ignore for `serial_test`\nsemver-major bumps, and a broad **`Cargo.lock`** refresh with\n**`docker/login-action@v4.6.0`**. Workflow regression tests assert\ntarpaulin timeout and wasm-bindgen lock alignment.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n2855d350eeaa818d518e3ea85dd4a62d339d9807. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-01T15:56:54-07:00",
          "tree_id": "7ff864f4c78d16393dabc944a3b64b1651e14682",
          "url": "https://github.com/wallstop/fortress-rollback/commit/88b6c3040482e05118caf56279b638dbd8ad3982"
        },
        "date": 1785625535439,
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
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 65,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 160,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 532,
            "range": "± 14",
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
            "value": 24,
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
            "value": 134,
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
            "value": 186,
            "range": "± 3",
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
            "value": 377,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 349,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 480,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 711,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 183,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 213,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 261,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 354,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 427,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 560,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 653,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 785,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1050,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 483,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 664,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 890,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1106,
            "range": "± 12",
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
            "value": 165,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 442,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 725,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1024,
            "range": "± 15",
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
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 82,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 304,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1439,
            "range": "± 9",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "49699333+dependabot[bot]@users.noreply.github.com",
            "name": "dependabot[bot]",
            "username": "dependabot[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "158e85cac5456898b1dfa22813eab77917ce9c2a",
          "message": "chore(deps): update serde in loom test lockfile (#259)\n\nBumps the cargo-loom-tests group with 1 update in the /loom-tests\ndirectory: [serde](https://github.com/serde-rs/serde).\n\nUpdates `serde` from 1.0.228 to 1.0.229\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/serde-rs/serde/releases\">serde's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v1.0.229</h2>\n<ul>\n<li>Update to syn 3</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/7fc3b4c30c94f73a96ebd1553f2b090d928fc3a8\"><code>7fc3b4c</code></a>\nRelease 1.0.229</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/6d6e9a11101354ce769a3438a088b6b9305c1863\"><code>6d6e9a1</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/serde-rs/serde/issues/3085\">#3085</a>\nfrom dtolnay/syn3</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/6dec3b751126c8338cac0fe8085612d695e4ecf3\"><code>6dec3b7</code></a>\nUpdate to syn 3</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/cfe669241065984177ff63af8b45058e6e9b499d\"><code>cfe6692</code></a>\nResolve mut_mut pedantic clippy lint</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/1023d077510b4aef36a41ef56fdb7798568a2654\"><code>1023d07</code></a>\nUpdate actions/upload-artifact@v6 -&gt; v7</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/dd682c2c86aa7629e77c1ccd93212d3729f4c66d\"><code>dd682c2</code></a>\nUpdate actions/checkout@v6 -&gt; v7</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/5f0f18b9211732f2d82f73b5a43e4f5ff3701251\"><code>5f0f18b</code></a>\nUpdate ui test suite to nightly-2026-06-01</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/63a1498f0e7be991ffac5939bdd202ca16e9a23f\"><code>63a1498</code></a>\nRegenerate stderr with trybuild normalization fixes</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/fa7da4a93567ed347ad0735c28e439fca688ef26\"><code>fa7da4a</code></a>\nFix unused_features warning</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/6b1a17851ea3d86a56aa116ca1cbf428f8d5f22d\"><code>6b1a178</code></a>\nUnpin CI miri toolchain</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/serde-rs/serde/compare/v1.0.228...v1.0.229\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Lockfile-only dependency patch with no changes to fortress-rollback or\nloom test code; serde 1.0.229 is a minor release updating serde_derive’s\nsyn dependency.\n> \n> **Overview**\n> Updates **`loom-tests/Cargo.lock`** only: **`serde`**,\n**`serde_core`**, and **`serde_derive`** move from **1.0.228** to\n**1.0.229**.\n> \n> The notable transitive change is that **`serde_derive`** now depends\non **`syn` 3.0.3** (added as a separate lockfile entry), while other\nproc-macro crates in the same lockfile keep **`syn` 2.0.111** via\nexplicit version pins. No application or test source files are modified.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n5389ad653105568c3a26ee765419bacdf6082db1. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-01T17:38:58-07:00",
          "tree_id": "b8e096eb90d3d694f36980260b35db06d983f2e7",
          "url": "https://github.com/wallstop/fortress-rollback/commit/158e85cac5456898b1dfa22813eab77917ce9c2a"
        },
        "date": 1785631649523,
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
            "value": 60,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 161,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 581,
            "range": "± 3",
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
            "value": 31,
            "range": "± 1",
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
            "value": 97,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 124,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 167,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 162,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 227,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 346,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 294,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 419,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 672,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 158,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 183,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 234,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 289,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 350,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 475,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 542,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 664,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 940,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 462,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 628,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 845,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1057,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 134,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 169,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 476,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 762,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1148,
            "range": "± 21",
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
            "value": 88,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 332,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1373,
            "range": "± 3",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "49699333+dependabot[bot]@users.noreply.github.com",
            "name": "dependabot[bot]",
            "username": "dependabot[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e3e90605b92fd68e593859286eba2f8f82192c31",
          "message": "chore(deps): refresh fuzz dependency lockfile (#268)\n\nBumps the cargo-fuzz group with 5 updates in the /fuzz directory:\n\n| Package | From | To |\n| --- | --- | --- |\n| [libfuzzer-sys](https://github.com/rust-fuzz/libfuzzer) | `0.4.10` |\n`0.4.13` |\n| [serde](https://github.com/serde-rs/serde) | `1.0.228` | `1.0.229` |\n| [tracing](https://github.com/tokio-rs/tracing) | `0.1.43` | `0.1.44` |\n| [smallvec](https://github.com/servo/rust-smallvec) | `1.15.1` |\n`1.15.2` |\n| [tracing-subscriber](https://github.com/tokio-rs/tracing) | `0.3.22` |\n`0.3.23` |\n\n\nUpdates `libfuzzer-sys` from 0.4.10 to 0.4.13\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/rust-fuzz/libfuzzer/blob/main/CHANGELOG.md\">libfuzzer-sys's\nchangelog</a>.</em></p>\n<blockquote>\n<h2>0.4.13</h2>\n<p>Released 2026-06-04.</p>\n<h3>Changed</h3>\n<ul>\n<li>Updated docs for the <code>fuzz_mutator!</code> macro to link to <a\nhref=\"https://docs.rs/mutatis\">the <code>mutatis</code>\ncrate</a>, which is a helpful crate when writing complex\ncustom mutators.</li>\n</ul>\n<hr />\n<h2>0.4.12</h2>\n<p>Released 2026-02-10.</p>\n<h3>Changed</h3>\n<ul>\n<li>Recommend <code>SmallRng</code> over <code>StdRng</code> in the\nexamples for faster, more lightweight\nseeding and sampling</li>\n<li>Updated <code>rand</code> dependency from 0.8.5 to 0.10</li>\n<li>Updated <code>flate2</code> dependency from 1.0.24 to 1.1</li>\n<li>Rename <code>gen</code> variable to <code>rng</code> for better 2024\nEdition compatibility</li>\n</ul>\n<hr />\n<h2>0.4.11</h2>\n<p>Released 2026-02-10.</p>\n<h3>Changed</h3>\n<ul>\n<li>Updated to <code>libFuzzer</code> commit <code>a47b42eb9f9b</code>\n(<code>release/22.x</code>).</li>\n</ul>\n<hr />\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/rust-fuzz/libfuzzer/commit/719e4efb9b8857ebaa782ae59376c8cbb78fed0f\"><code>719e4ef</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/rust-fuzz/libfuzzer/issues/144\">#144</a>\nfrom fitzgen/bump-to-0.4.13</li>\n<li><a\nhref=\"https://github.com/rust-fuzz/libfuzzer/commit/2500b23c78391c022f57ff1ee359b186ad48464d\"><code>2500b23</code></a>\nBump to version 0.4.13</li>\n<li><a\nhref=\"https://github.com/rust-fuzz/libfuzzer/commit/e9120b05b4d85d4c3d70a98f4c9eafce6d6d60d0\"><code>e9120b0</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/rust-fuzz/libfuzzer/issues/143\">#143</a>\nfrom fitzgen/link-to-mutatis-from-fuzz-mutator</li>\n<li><a\nhref=\"https://github.com/rust-fuzz/libfuzzer/commit/0743afb84597680abf52881307a9c37ae9eadbf0\"><code>0743afb</code></a>\nLink to <code>mutatis</code> from the <code>fuzz_mutator!</code>\ndocs</li>\n<li><a\nhref=\"https://github.com/rust-fuzz/libfuzzer/commit/752f2e26ba609c1d909795ff0e9bf9685c9d410b\"><code>752f2e2</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/rust-fuzz/libfuzzer/issues/142\">#142</a>\nfrom fitzgen/bump-to-0.4.12</li>\n<li><a\nhref=\"https://github.com/rust-fuzz/libfuzzer/commit/735448b1416f0326f81d54b92cf47d1052180bbf\"><code>735448b</code></a>\nBump to 0.4.12</li>\n<li><a\nhref=\"https://github.com/rust-fuzz/libfuzzer/commit/be9241f1b73b802f44d17f622c39f03a50ff2fa9\"><code>be9241f</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/rust-fuzz/libfuzzer/issues/140\">#140</a>\nfrom rchildre3/update-deps-and-rand-algos</li>\n<li><a\nhref=\"https://github.com/rust-fuzz/libfuzzer/commit/4b013b634b8b89b146caaacd4d578cc7e2b0f28e\"><code>4b013b6</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/rust-fuzz/libfuzzer/issues/141\">#141</a>\nfrom fitzgen/bump-to-version-0.4.11</li>\n<li><a\nhref=\"https://github.com/rust-fuzz/libfuzzer/commit/312386d095b11bc7d610e1d245c8ae76b729f4d8\"><code>312386d</code></a>\nBump to version 0.4.11</li>\n<li><a\nhref=\"https://github.com/rust-fuzz/libfuzzer/commit/31764e336c54023817e7d5d4d94d19bf794530c2\"><code>31764e3</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/rust-fuzz/libfuzzer/issues/139\">#139</a>\nfrom rchildre3/update-libfuzzer-22.x</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/rust-fuzz/libfuzzer/compare/0.4.10...0.4.13\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\nUpdates `serde` from 1.0.228 to 1.0.229\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/serde-rs/serde/releases\">serde's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v1.0.229</h2>\n<ul>\n<li>Update to syn 3</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/7fc3b4c30c94f73a96ebd1553f2b090d928fc3a8\"><code>7fc3b4c</code></a>\nRelease 1.0.229</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/6d6e9a11101354ce769a3438a088b6b9305c1863\"><code>6d6e9a1</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/serde-rs/serde/issues/3085\">#3085</a>\nfrom dtolnay/syn3</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/6dec3b751126c8338cac0fe8085612d695e4ecf3\"><code>6dec3b7</code></a>\nUpdate to syn 3</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/cfe669241065984177ff63af8b45058e6e9b499d\"><code>cfe6692</code></a>\nResolve mut_mut pedantic clippy lint</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/1023d077510b4aef36a41ef56fdb7798568a2654\"><code>1023d07</code></a>\nUpdate actions/upload-artifact@v6 -&gt; v7</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/dd682c2c86aa7629e77c1ccd93212d3729f4c66d\"><code>dd682c2</code></a>\nUpdate actions/checkout@v6 -&gt; v7</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/5f0f18b9211732f2d82f73b5a43e4f5ff3701251\"><code>5f0f18b</code></a>\nUpdate ui test suite to nightly-2026-06-01</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/63a1498f0e7be991ffac5939bdd202ca16e9a23f\"><code>63a1498</code></a>\nRegenerate stderr with trybuild normalization fixes</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/fa7da4a93567ed347ad0735c28e439fca688ef26\"><code>fa7da4a</code></a>\nFix unused_features warning</li>\n<li><a\nhref=\"https://github.com/serde-rs/serde/commit/6b1a17851ea3d86a56aa116ca1cbf428f8d5f22d\"><code>6b1a178</code></a>\nUnpin CI miri toolchain</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/serde-rs/serde/compare/v1.0.228...v1.0.229\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\nUpdates `tracing` from 0.1.43 to 0.1.44\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/tokio-rs/tracing/releases\">tracing's\nreleases</a>.</em></p>\n<blockquote>\n<h2>tracing 0.1.44</h2>\n<h3>Fixed</h3>\n<ul>\n<li>Fix <code>record_all</code> panic (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>)</li>\n</ul>\n<h3>Changed</h3>\n<ul>\n<li><code>tracing-core</code>: updated to 0.1.36 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>)</li>\n</ul>\n<p><a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/pull/3432\">tokio-rs/tracing#3432</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/pull/3440\">tokio-rs/tracing#3440</a></p>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/2d55f6faf9be83e7e4634129fb96813241aac2b8\"><code>2d55f6f</code></a>\nchore: prepare tracing 0.1.44 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3439\">#3439</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/10a9e838a35e6ded79d66af246be2ee05417136d\"><code>10a9e83</code></a>\nchore: prepare tracing-core 0.1.36 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/ee82cf92a8c750f98cfb7a417cc8defb37e26a00\"><code>ee82cf9</code></a>\ntracing: fix record_all panic (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/9978c3663bcd58de14b3cf089ad24cb63d00a922\"><code>9978c36</code></a>\nchore: prepare tracing-mock 0.1.0-beta.3 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3429\">#3429</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/cc44064b3a41cb586bd633f8a024354928e25819\"><code>cc44064</code></a>\nchore: prepare tracing-subscriber 0.3.22 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3428\">#3428</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/tokio-rs/tracing/compare/tracing-0.1.43...tracing-0.1.44\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\nUpdates `smallvec` from 1.15.1 to 1.15.2\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/servo/rust-smallvec/releases\">smallvec's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v1.15.2</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Exclude development script from cargo packaging by <a\nhref=\"https://github.com/weiznich\"><code>@​weiznich</code></a> in <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/pull/397\">servo/rust-smallvec#397</a></li>\n<li>Fix use-after-free in DrainFilter::keep_rest for SmallVec with\ncapacity 0 by <a\nhref=\"https://github.com/Shnatsel\"><code>@​Shnatsel</code></a> in <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/pull/407\">servo/rust-smallvec#407</a></li>\n<li>Work around rustc 1.93 perf regression with MaybeUninit by <a\nhref=\"https://github.com/glandium\"><code>@​glandium</code></a> in <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/pull/410\">servo/rust-smallvec#410</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/Shnatsel\"><code>@​Shnatsel</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/pull/407\">servo/rust-smallvec#407</a></li>\n<li><a href=\"https://github.com/glandium\"><code>@​glandium</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/pull/410\">servo/rust-smallvec#410</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/servo/rust-smallvec/compare/v1.15.1...v1.15.2\">https://github.com/servo/rust-smallvec/compare/v1.15.1...v1.15.2</a></p>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/c469051a1ba05ef1a03dd69e14b4a5aa329e6f10\"><code>c469051</code></a>\nBump version.</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/9fe422b9cd1ab6350e35ca48386a5de348900583\"><code>9fe422b</code></a>\nFix Windows CI.</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/51b965f56a066888828dae0b84e2ed190a1bdfe7\"><code>51b965f</code></a>\nWork around rustc 1.93 perf regression with MaybeUninit</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/9da26a5315c563d4de181b0be9e75d165289f81e\"><code>9da26a5</code></a>\nFix use-after-free in DrainFilter::keep_rest for zero-capacity\nSmallVecs</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/79184f15937f841881cb32f02fe30286def5b69b\"><code>79184f1</code></a>\nAdd Miri test for use-after-free in DrainFilter::keep_rest</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/f59fb36b35064e63aa74992aab807552b1b68096\"><code>f59fb36</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/issues/397\">#397</a>\nfrom GiGainfosystems/exclude_scripts</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/28b6ed71755c929634fc331c30a1d32b95edf576\"><code>28b6ed7</code></a>\nExclude development script</li>\n<li>See full diff in <a\nhref=\"https://github.com/servo/rust-smallvec/compare/v1.15.1...v1.15.2\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\nUpdates `tracing-subscriber` from 0.3.22 to 0.3.23\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/tokio-rs/tracing/releases\">tracing-subscriber's\nreleases</a>.</em></p>\n<blockquote>\n<h2>tracing-subscriber 0.3.23</h2>\n<h3>Fixed</h3>\n<ul>\n<li>Allow ansi sanitization to be disabled (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3484\">#3484</a>)</li>\n</ul>\n<p><a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3484\">#3484</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/pull/3484\">tokio-rs/tracing#3484</a></p>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/54ede4d5d85a536aed5485c5213011d9ec961935\"><code>54ede4d</code></a>\nchore: prepare tracing-subscriber 0.3.23 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3490\">#3490</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/37558d5f26340e999089bf3a680a800435332312\"><code>37558d5</code></a>\nsubscriber: allow ansi sanitization to be disabled (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3484\">#3484</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/efc690fa6bd1d9c3a57528b9bc8ac80504a7a6ed\"><code>efc690f</code></a>\ncore: add missing const (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3449\">#3449</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/0c32367cf9df27e750c4c81803de62a4e64e2ef1\"><code>0c32367</code></a>\ncore: Use const initializers instead of <code>once_cell</code></li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/9feb241133e55e70c7d4399689b8ef72f71d070f\"><code>9feb241</code></a>\ndocs: add arcswap reload crate to related (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3442\">#3442</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/2d55f6faf9be83e7e4634129fb96813241aac2b8\"><code>2d55f6f</code></a>\nchore: prepare tracing 0.1.44 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3439\">#3439</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/10a9e838a35e6ded79d66af246be2ee05417136d\"><code>10a9e83</code></a>\nchore: prepare tracing-core 0.1.36 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/ee82cf92a8c750f98cfb7a417cc8defb37e26a00\"><code>ee82cf9</code></a>\ntracing: fix record_all panic (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/9978c3663bcd58de14b3cf089ad24cb63d00a922\"><code>9978c36</code></a>\nchore: prepare tracing-mock 0.1.0-beta.3 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3429\">#3429</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/tokio-rs/tracing/compare/tracing-subscriber-0.3.22...tracing-subscriber-0.3.23\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Lockfile-only dependency bumps in the fuzz harness; no production code\npaths changed, though serde’s syn 3 move is worth a quick fuzz build\nsmoke check.\n> \n> **Overview**\n> Updates **`fuzz/Cargo.lock`** only—no Rust source changes. Five direct\nbumps in the fuzz workspace: **`libfuzzer-sys`** 0.4.10→0.4.13,\n**`serde`** 1.0.228→1.0.229, **`tracing`** 0.1.43→0.1.44, **`smallvec`**\n1.15.1→1.15.2, and **`tracing-subscriber`** 0.3.22→0.3.23.\n> \n> The lockfile also pins **`syn`** twice (`2.0.111` for most\nproc-macros, **`3.0.3`** for **`serde_derive`** after serde’s syn 3\nmigration), plus transitive bumps to **`tracing-core`**,\n**`serde_core`**, and **`serde_derive`**.\n> \n> Notable upstream fixes in these versions: **`smallvec`**\nuse-after-free in `DrainFilter::keep_rest`, **`tracing`** `record_all`\npanic fix, and **`libfuzzer-sys`** libFuzzer 22.x / dependency updates\nin 0.4.11–0.4.13.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n516d246095bcd69eadbd0ab8a9a5cfbb22600baa. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-01T18:24:00-07:00",
          "tree_id": "77500f93ec513809f5f13c53751c8a12e9a7018d",
          "url": "https://github.com/wallstop/fortress-rollback/commit/e3e90605b92fd68e593859286eba2f8f82192c31"
        },
        "date": 1785634366602,
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
            "value": 26,
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
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 93,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 337,
            "range": "± 13",
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
            "value": 46,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 63,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 159,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 575,
            "range": "± 18",
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
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 102,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 127,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 167,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 174,
            "range": "± 3",
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
            "value": 324,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 318,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 426,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 623,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 159,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 189,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 222,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 289,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 349,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 440,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 574,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 697,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 875,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 554,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 654,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 817,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 987,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 99,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 135,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 574,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 968,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1481,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 11,
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
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 84,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 283,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1056,
            "range": "± 17",
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
          "id": "fc22bc4af27e9e5d331632183b1be54494938ae0",
          "message": "test: add deterministic allocation contracts (#270)\n\nEstablish an isolated, deterministic heap-allocation ledger for documented zero-allocation paths and warmed sync-test ceilings.\n\nProgresses #264.",
          "timestamp": "2026-08-01T19:21:20-07:00",
          "tree_id": "ed3c0306b69ae30b92cc28668e9198b899b7de6f",
          "url": "https://github.com/wallstop/fortress-rollback/commit/fc22bc4af27e9e5d331632183b1be54494938ae0"
        },
        "date": 1785637793881,
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
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 64,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 159,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 530,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 25,
            "range": "± 1",
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
            "value": 131,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 171,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 186,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 251,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 380,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 347,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 475,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 711,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 183,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 210,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 255,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 353,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 424,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 551,
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
            "value": 779,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1039,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 483,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 685,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 885,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1087,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 113,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 165,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 439,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 711,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1025,
            "range": "± 11",
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 305,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1437,
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
          "id": "df198e728fb3b87e0ba5cd3914bb65f0d07479d0",
          "message": "chore(ci): refresh current action pins (#271)\n\nUpdate all sccache-action call sites to v0.0.11 and pin setup-java to v5.7.0 while preserving the publish workflow's immutable SHA policy.\n\nSupersedes #269.",
          "timestamp": "2026-08-01T19:48:51-07:00",
          "tree_id": "6711ce26741e8a72cf3e891561326fe757905de0",
          "url": "https://github.com/wallstop/fortress-rollback/commit/df198e728fb3b87e0ba5cd3914bb65f0d07479d0"
        },
        "date": 1785639446798,
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
            "value": 83,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 279,
            "range": "± 5",
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
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 65,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 160,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 531,
            "range": "± 1",
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
            "value": 138,
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
            "value": 185,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 268,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 381,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 347,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 473,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 721,
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
            "value": 254,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 353,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 436,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 549,
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
            "value": 803,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1038,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 481,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 628,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 833,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1099,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 113,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 165,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 441,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 701,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1018,
            "range": "± 12",
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 303,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1434,
            "range": "± 7",
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
          "id": "48532d1f5613cc3a4528cffc0ba76199f16dd8bc",
          "message": "Eliminate warmed sync-test input staging allocations (#272)\n\n## What\n\n- replace per-frame `BTreeMap` input staging in `SyncTestSession` with\nfallibly preallocated player-indexed slots\n- retain slot storage across frames while preserving duplicate\noverwrite, missing-input retry, and deterministic player ordering\n- tighten the deterministic allocation ledger to zero operations/bytes\nfor warmed N=2 and N=4 frames, and one exact 128-byte\nreturned-`InputVec` spill at N=16\n- document the observable performance improvement in the changelog\n\n## Why\n\nPR #270 established a deterministic allocation ledger and measured one\nallocation / 192 bytes per warmed frame at N=2 and N=4, plus four\noperations / 800 bytes at N=16. Those allocations came from rebuilding\nthe sync-test staging map every frame. Constructor-owned slots remove\nthat repeated work without changing the public API or wire behavior.\n\nProgresses #264.\n\n## Impact\n\nWarmed `SyncTestSession::advance_frame()` calls for one-to-four-player\nsessions are heap-allocation-free. Sixteen-player frames now allocate\nonly for the returned `InputVec` above its inline capacity. Construction\nremains fallible for arbitrary configured player counts, and an\nimpossible slot-length mismatch returns a structured internal error.\n\n## Validation\n\n- allocation contract: 10 consecutive debug runs and one release run\n- focused semantic/allocation tests: 3/3\n- sync-test unit suite: 49/49\n- sync-test determinism regression: 1/1\n- default Nextest matrix: 2,883/2,883\n- `hot-join` Nextest matrix: 3,139/3,139\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- warning-denied workspace documentation\n- changelog formatting, Unreleased policy, and version synchronization\n- agent preflight, including 286 release-automation tests and 1,393\nlinks\n- independent adversarial review: zero findings\n\n## Review Readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS\n- Determinism: PASS\n- Agent preflight: PASS\n- Error handling: PASS\n- Tests breadth: PASS\n- Design log reviewed: N/A (private staging refactor)\n- CHANGELOG reviewed: YES\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Private sync-test staging only; semantics are covered by unit and\nallocation-contract tests with no public API or network wire changes.\n> \n> **Overview**\n> **`SyncTestSession`** no longer rebuilds per-frame input staging with\na **`BTreeMap`**. Construction fallibly reserves\n**`Vec<Option<PlayerInput>>`** indexed by player handle;\n**`add_local_input`** overwrites slots, and **`advance_frame`** clears\nslots to **`None`** while keeping that storage for the next frame.\n> \n> **Behavior preserved or tightened:** missing-input checks cover empty\nslots; player-index ordering in **`AdvanceFrame`** inputs is unchanged;\na failed **`advance_frame`** keeps already-staged inputs so callers can\nadd only missing players and retry.\n> \n> **Allocation impact:** warmed **`advance_frame()`** for 1–4 players is\nheap-free; 16-player frames are limited to the returned **`InputVec`**\nspill above inline capacity (allocation contract and changelog updated).\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\ne75047918c901b6ab1d3efd941014a20544284f7. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-01T21:10:58-07:00",
          "tree_id": "9ec5f320b3389a097eb8fdaa21a7fc088ad6c7a0",
          "url": "https://github.com/wallstop/fortress-rollback/commit/48532d1f5613cc3a4528cffc0ba76199f16dd8bc"
        },
        "date": 1785644435287,
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
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 65,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 160,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 532,
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
            "value": 134,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 178,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 186,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 264,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 376,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 347,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 480,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 721,
            "range": "± 6",
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
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 257,
            "range": "± 12",
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
            "value": 429,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 549,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 653,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 787,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1038,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 484,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 620,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 841,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1055,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 94,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 130,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 440,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 681,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1093,
            "range": "± 120",
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
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 303,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1427,
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
          "id": "0d60cf4b90ff8624d8036316f607cae7a6ce2946",
          "message": "Eliminate P2P local-input staging allocations (#275)\n\n## What\n\n- replace per-frame `P2PSession` local-input `BTreeMap` staging with\nfallibly preallocated player-indexed slots\n- retain slot storage across frames while preserving sparse handles,\noverwrite, retry, input-drop, and deterministic ordering semantics\n- let protocol serialization consume map-backed or slot-backed inputs\nwithout changing canonical handle order or encoded bytes\n- extend the deterministic allocation ledger to the isolated warmed\nall-local P2P staging path\n\n## Why\n\nIssue #264's allocation ledger identified the next candidate but\nrequired measurement before optimization. The zero-allocation target\nfailed with a stable signature:\n\n- N=2: one allocation / 192 bytes\n- N=4: one allocation / 192 bytes\n- N=16: four allocations / 800 bytes\n\nThe transient cost matched the removed sync-test staging map: N=16\ncontained 672 bytes of map nodes plus the returned `InputVec`'s 128-byte\nspill.\n\n## Result\n\nWith desync detection disabled to isolate staging, the warm-up save\nrequest fulfilled, and application callbacks outside the measured\nregion:\n\n- N=2: zero allocations / zero bytes\n- N=4: zero allocations / zero bytes\n- N=16: one allocation / 128 bytes (the returned `InputVec` spill)\n\nNetwork encoding and checksum history retain their separate bounded\nallocations; this PR does not claim whole networked frames are\nallocation-free.\n\n## Semantics and safety\n\nRegression coverage pins:\n\n- sparse/nonzero local player handles\n- reverse submission preserving ascending player order\n- observable duplicate overwrite\n- missing-input failure followed by retry with only the missing slot\n- `Frame::NULL` local-input rejection suppressing the whole endpoint\nsend\n- map-backed and slot-backed input sources producing identical frame\nmetadata and bytes\n- fallible construction-time reservation with structured errors\n\nNo public signature, config default, or wire layout changes.\n\n## Validation\n\n- allocation target went red with the exact baseline above before\nproduction changes\n- ten consecutive debug allocation runs\n- release-mode allocation contract\n- default Nextest: 2,886 passed, 71 skipped\n- hot-join Nextest: 3,142 passed, 72 skipped\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- warning-denied workspace docs\n- changelog/version checks, 1,393 links, 286 release-automation tests,\nspelling, allocation-bound hook, and full agent preflight\n- independent adversarial review: zero remaining findings\n\n## Review readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS\n- Determinism: PASS\n- Agent preflight: PASS\n- Error handling: PASS\n- Tests breadth: PASS\n- Design log reviewed: N/A (private staging representation)\n- CHANGELOG reviewed: YES\n\nProgresses #264.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Internal staging and protocol abstraction only; no public API or wire\nlayout changes. Risk is mainly behavioral parity (sparse handles, drop\nsuppression, encoding order), which the expanded tests target.\n> \n> **Overview**\n> **P2P local-input staging** moves from a per-frame `BTreeMap` to\nconstructor-reserved `Vec<Option<PlayerInput>>` slots indexed by player\nhandle. Slots are cleared with `None` after each advance instead of\n`clear()`, so sparse handles, overwrite, and missing-input retry keep\nworking without rebuilding map nodes each frame.\n> \n> **Wire encoding** gains an internal `InputSource` trait so\n`InputBytes::try_from_inputs` and `send_input` accept either the old map\nor the new slot slice; ascending handle order and encoded bytes stay\nidentical.\n> \n> **Allocation contract** adds warmed all-local `P2PSession` frame\nmeasurements (desync off, warm-up save fulfilled): 2–4 players stay\nheap-free on staging; 16 players keep only the returned `InputVec`\nspill. Network encode and checksum history are outside that ledger.\n> \n> Regression tests cover sparse local handles, dropped-input send\nsuppression, map vs slot encoding parity, and overwrite/advance\nbehavior.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n95a01679fdb10aeaad5ad03f748ba8dc30ad25e0. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-02T09:03:21-07:00",
          "tree_id": "374b3a2b6007454278e38b596c04be67b05d3d87",
          "url": "https://github.com/wallstop/fortress-rollback/commit/0d60cf4b90ff8624d8036316f607cae7a6ce2946"
        },
        "date": 1785687122307,
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
            "range": "± 5",
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
            "value": 41,
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
            "value": 579,
            "range": "± 5",
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
            "range": "± 1",
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
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 97,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 125,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 167,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 162,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 227,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 346,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 294,
            "range": "± 2",
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
            "value": 651,
            "range": "± 2",
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
            "value": 181,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 227,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 289,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 351,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 474,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 542,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 668,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 939,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 462,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 631,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 849,
            "range": "± 99",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1044,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 102,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 137,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 436,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 697,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1049,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 21,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 85,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 356,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1298,
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
          "id": "71d9ca03f7aa05528793b0a6bf4ce01fe40f66fa",
          "message": "Refresh standalone workspace dependencies (#276)\n\n## Summary\n\n- refresh every dependency in the three standalone Cargo workspaces to\nthe newest version allowed by their manifests\n- add the previously omitted `/tests/godot-emscripten` workspace to\nweekly Dependabot coverage\n- leave the root workspace unchanged because `cargo update --workspace`\nreports zero compatible updates\n\n## Compatibility and security\n\n- retains the repository's declared dependency constraints and lockfile\nformat\n- fuzz and Loom workspaces compile with Rust 1.86\n- the Godot Emscripten probe compiles on its supported current toolchain\nfor `wasm32-unknown-emscripten`; its existing `godot 0.5.4` dependency\nrequires Rust 1.94\n- `cargo audit` reports no vulnerabilities or yanked crates; the allowed\nbincode unmaintained advisory remains tracked in #273\n- the macroquad advisory remains tracked separately in #274\n\n## Validation\n\n- `cargo +1.86.0 check --locked --manifest-path fuzz/Cargo.toml`\n- `RUSTFLAGS='--cfg loom' cargo +1.86.0 check --locked --manifest-path\nloom-tests/Cargo.toml`\n- `cargo check --locked --manifest-path\ntests/godot-emscripten/Cargo.toml --target wasm32-unknown-emscripten`\n- `cargo audit --file` for all three refreshed lockfiles\n- `cargo update --dry-run` for all three standalone manifests: zero\nremaining compatible updates\n- repository pre-commit hooks for the staged files\n\n## Review readiness\n\nThis is intentionally separate from #275: it contains only generated\nlockfile refreshes and Dependabot coverage. It has no library source,\npublic API, protocol, or behavior change.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Dependency-only lockfile and CI config changes with no API or runtime\nbehavior changes; risk is limited to toolchain/test compatibility if a\ntransitive update regresses builds.\n> \n> **Overview**\n> Refreshes **Cargo.lock** for the three standalone workspaces (`fuzz/`,\n`loom-tests/`, and `tests/godot-emscripten/`) to the newest versions\nallowed by their manifests, without changing root workspace dependencies\nor any library source.\n> \n> Lockfile churn is mostly transitive bumps (e.g. `syn`, `libc`,\n`wasm-bindgen`, `futures-*`, regex crates) and slimmer Windows-related\ndependency trees where upstream crates dropped older `windows` umbrella\npackages.\n> \n> **Dependabot** now includes weekly grouped updates for\n`/tests/godot-emscripten`, matching the other isolated Cargo workspaces.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n51e40e2a0e3188c369815074b61dee3b6c56021a. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-02T16:04:28Z",
          "tree_id": "266f1aefcfa67aaa6c9142447b07fab320313618",
          "url": "https://github.com/wallstop/fortress-rollback/commit/71d9ca03f7aa05528793b0a6bf4ce01fe40f66fa"
        },
        "date": 1785687495135,
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
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 49,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 65,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 160,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 531,
            "range": "± 1",
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
            "value": 134,
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
            "value": 186,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 262,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 374,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 347,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 474,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 741,
            "range": "± 7",
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
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 265,
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
            "value": 425,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 550,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 652,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 789,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1064,
            "range": "± 8",
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
            "value": 637,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 851,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1052,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 96,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 132,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 407,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 653,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 974,
            "range": "± 25",
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
            "value": 304,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1410,
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
          "id": "79cb86dd4c5011feecc4d4cdf9e7dd265ef8e6eb",
          "message": "Eliminate empty graceful-drop poll reservations (#277)\n\n## Summary\n\n- add a synchronized one-endpoint P2P allocation contract at N=2, N=4,\nand N=16\n- stop ordinary P2P polling from reserving 256 graceful-drop records per\nremote when bounded endpoint mailboxes are empty\n- preserve queued and active coordinated-drop progress on empty polls,\nwith a direct regression test\n- document the measured improvement and close the broad\nbenchmark/allocation research issue\n\n## Root cause\n\n`P2PSession::poll_coordinated_drop` reserved `remote_count *\nMAX_RECEIVE_MESSAGES_PER_POLL` entries on every poll. A warmed frame\ntherefore allocated 28,672 temporary bytes even when every endpoint's\nbounded graceful-drop mailbox was empty.\n\nThe poll now checked-sums the exact staged message counts and reserves\nonly that amount. Arithmetic overflow and allocation failure still fail\nclosed with `ResourceLimit`; an empty count does not skip the lifecycle\ndriver.\n\n## Measured impact\n\n| Players | Before | After | Contract |\n| ---: | ---: | ---: | ---: |\n| 2 | 5 ops / 28,717 B | 4 ops / 45 B | 4 ops / 128 B |\n| 4 | 5 ops / 28,729 B | 4 ops / 57 B | 4 ops / 128 B |\n| 16 | 6 ops / 29,050 B | 5 ops / 378 B | 5 ops / 512 B |\n\nThe fixture proves synchronization and acknowledged warm-up, exactly one\n`Input` submission, all local-player input bytes, delta/RLE compression,\none pending bounded frame, one application advance, and no rollback\nrequest.\n\n## Validation\n\n- `cargo fmt --check`\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- default Nextest: 2,887 passed, 71 skipped\n- hot-join Nextest: 3,143 passed, 72 skipped\n- warning-denied workspace rustdoc\n- doctests: 160 passed, 50 ignored\n- allocation contract: 10 complete debug repetitions plus release\n- repository agent preflight, changelog check, version-sync check,\nMarkdown lint, and link check\n- three-round adversarial review; findings were corrected in the fixture\nand empty-mailbox lifecycle regression\n\nCloses #264.\n\n## Follow-up disposition\n\n#242 remains externally gated by upstream Signal Fish browser E2E work.\nDependency research remains tracked separately by #273 (bincode\nreplacement) and #274 (macroquad example-stack security/replacement);\nthis PR changes no dependency or wire format.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Localized polling/allocation change with regression and\nallocation-contract coverage; coordinated-drop fail-closed paths on\noverflow remain unchanged.\n> \n> **Overview**\n> **`poll_coordinated_drop`** no longer reserves `remote_count × 256`\ngraceful-drop slots on every frame poll when endpoint mailboxes are\nempty. It sums each remote’s staged drop count via new\n**`UdpProtocol::received_drop_message_count`**, reserves only that exact\ncapacity (skipping reservation when zero), and still fails closed on\noverflow or allocation failure.\n> \n> A unit test confirms **empty mailboxes still advance** queued\ncoordinated-drop work (single-survivor commit without inbound control\nmessages). The **allocation contract** adds a warmed two-session\nnetworked send fixture at N=2/4/16 with byte ceilings (45–378 B vs ~29\nKiB before), documenting the fix in **CHANGELOG**.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n086bb2e1f6360c6fc7a56b3bccb64a55e26bfa54. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-02T10:14:05-07:00",
          "tree_id": "3f73b33f4e34d1fa6326eadd6edce301044aaf3c",
          "url": "https://github.com/wallstop/fortress-rollback/commit/79cb86dd4c5011feecc4d4cdf9e7dd265ef8e6eb"
        },
        "date": 1785691376007,
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
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 307,
            "range": "± 19",
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
            "value": 60,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 160,
            "range": "± 1",
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
            "value": 33,
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
            "value": 126,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 168,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 162,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 228,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 346,
            "range": "± 0",
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 673,
            "range": "± 3",
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
            "value": 184,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 228,
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
            "value": 351,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 475,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 545,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 661,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 939,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 463,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 600,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 801,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1002,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 102,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 139,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 439,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 686,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1058,
            "range": "± 17",
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
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 87,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 330,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1375,
            "range": "± 15",
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
          "id": "bbabac808dd381a6835627c134bf7c90583fc4bf",
          "message": "Make dropped-input Miri regression deterministic (#279)\n\n## Summary\n\n- fix the timing-dependent dropped-input regression that broke macOS\nMiri on `main`\n- inject a deterministic protocol clock and deliberately cross the\nquality-report interval\n- assert the real contract—no `Input` message—while proving legitimate\ntimer traffic may coexist\n\nCloses #278.\n\n## Root cause\n\nThe regression added in #275 used the platform monotonic clock and\nrecorded every packet sent by the endpoint. It then asserted that the\nentire socket remained empty across `P2PSession::advance_frame()`.\n\nThat assertion was broader than the behavior under test.\n`advance_frame()` first calls `poll_remote_clients()`; a running\nendpoint may legitimately emit interval-gated control traffic there.\nSlow macOS Miri execution crossed the default 200 ms quality-report\ninterval, so the recorder contained a valid `QualityReport` even though\nthe dropped `Frame::NULL` local input correctly emitted no `Input`\npacket.\n\nThis reached `main` because PR #275's exact-head macOS Miri job did run\nthe test and passed. The wall-clock-sensitive oracle changed outcome\nwith runner/interpreter timing rather than code behavior.\n\n## Fix\n\nThe test now:\n\n1. injects a controlled `ProtocolConfig::clock`;\n2. advances it just beyond the configured quality-report interval;\n3. proves a legitimate `QualityReport` is present; and\n4. verifies that every recorded packet is non-`Input`.\n\nThis makes the former false failure deterministic and keeps the\nassertion mutation-sensitive to the production input-suppression guard.\n\nNo production code, public API, wire format, runtime behavior, or\ndependency changes are included.\n\n## Validation\n\n- `cargo fmt --check`\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`\n- default Nextest: 2,887 passed; 71 skipped\n- hot-join Nextest: 3,143 passed; 72 skipped\n- pinned `nightly-2026-02-10` Miri seed 0 targeted regression\n- `RUSTDOCFLAGS=\"-D warnings\" cargo doc --workspace --no-deps`\n- Rust 1.86 all-target check\n- agent preflight\n- dependency dry-runs for all four Cargo workspaces: zero compatible\nupdates\n\n## Review Readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS (test-only change)\n- Determinism: PASS (wall clock replaced with injected time)\n- Agent preflight: PASS\n- Error handling: N/A (no production change)\n- Tests breadth: PASS (ordinary + exact pinned Miri + full\ndefault/hot-join matrices)\n- Design log reviewed: N/A\n- CHANGELOG reviewed: N/A (test-oracle repair only)\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Test oracle and naming only; no runtime or public API changes.\n> \n> **Overview**\n> Repairs the **dropped local input** P2P session test so it no longer\nflakes on slow macOS Miri runs.\n> \n> The test now wires a **deterministic `ProtocolConfig::clock`**, bumps\ntime past the quality-report interval, and checks the real contract:\n**`Input` messages stay suppressed** when local input is dropped to\n`Frame::NULL`, while a legitimate **`QualityReport`** may still be sent\nduring `advance_frame()` polling. The old assertion that the recording\nsocket was completely empty was too strict and failed when\ninterval-gated control traffic crossed the default 200 ms window.\n> \n> **Test-only** change; no production, API, or wire-format updates.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n8dbafec579173522a78d81c1f310852649459e7d. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-02T11:09:17-07:00",
          "tree_id": "21cdd393562fb21bdb6692e6497abdc7f3110a84",
          "url": "https://github.com/wallstop/fortress-rollback/commit/bbabac808dd381a6835627c134bf7c90583fc4bf"
        },
        "date": 1785694671445,
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
            "range": "± 17",
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
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 65,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 160,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 531,
            "range": "± 2",
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
            "value": 175,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 186,
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
            "value": 377,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 348,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 480,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 722,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 182,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 212,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 260,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 353,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 425,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 549,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 652,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 780,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1040,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 491,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 624,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 844,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1057,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 94,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 132,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 409,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 682,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1002,
            "range": "± 20",
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
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 23,
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
            "value": 303,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1437,
            "range": "± 8",
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
          "id": "20caf394a1f157a4cfbe2877f6891f657a3184ad",
          "message": "security: remove advisory-affected graphical examples (#280)\n\nCloses #274\n\n## Summary\n\n- remove the Macroquad-based ExGame binaries and the advisory-affected\nMacroquad/font stack from every Cargo workspace lock\n- retain `graphical-examples` as an empty deprecated compatibility\nfeature so existing manifests keep resolving without enabling code or\ndependencies\n- remove demo-only dependencies, native graphical packages, stale\ncargo-vet exemptions, and feature-matrix exclusions; ban `macroquad` and\n`ttf-parser` from re-entry\n- keep and harden five portable headless examples, including fail-closed\nstructured error handling and browser-WASM custom-socket compilation\n- document the security boundary, migration path, and dependency design\ndecision\n\n## Security rationale\n\nMacroquad 0.4.16 exposes process-global mutable context through safe\nAPIs used by the deleted render loops and has no patched release for\nRUSTSEC-2025-0035. Its font initialization selected the unmaintained\n`ttf-parser` owner covered by RUSTSEC-2026-0192. Neither crate remains\nin the resolved graph or workspace locks.\n\n## Validation\n\n- default Nextest: 2,887 passed; 71 skipped\n- hot-join Nextest: 3,143 passed; 72 skipped\n- Rust 1.86 all-target check\n- strict workspace Clippy with `tokio,json`\n- all five retained examples compile and run\n- browser `wasm32-unknown-unknown` custom-socket example check\n- cargo-semver-checks: 196/196 checks passed against 0.11.0\n- `cargo audit`: only separately tracked bincode warning (#273)\n- `cargo deny check advisories licenses bans sources`\n- workspace lock/version sync, docs/wiki/link claims, actionlint,\nrelease/toolchain tests, and authoritative agent preflight\n- two independent adversarial review iterations; no blockers remain\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Removes public graphical binaries and changes the default example\nstory (breaking for users who relied on ex_game), but core library\nnetworking behavior is unchanged; supply-chain risk from the advisory\nstack is eliminated.\n> \n> **Overview**\n> Removes the Macroquad-based **ExGame** examples (`ex_game_p2p`,\n`ex_game_spectator`, `ex_game_synctest`) and the **Macroquad** optional\ndependency from the workspace lockfile because Macroquad has no patched\nrelease for soundness issues reachable from safe code\n(RUSTSEC-2025-0035) and pulled in unmaintained **ttf-parser**\n(RUSTSEC-2026-0192).\n> \n> **`graphical-examples`** stays as a **deprecated no-op** feature so\nexisting Cargo manifests keep resolving without enabling code or deps.\n**`cargo deny`** now **bans** `macroquad` and `ttf-parser` to block\nre-entry.\n> \n> **Supply chain & tooling:** Strips graphical dev-deps (`macroquad`,\n`clap`, `tracing-log`), Linux devcontainer graphics/audio packages,\nstale **cargo-vet** exemptions, and **`graphical-examples`** from\nmutation/cargo-hack feature exclusions.\n> \n> **Examples & CI:** README and docs pivot to **headless** examples\n(`configuration`, `custom_socket`, `error_handling`, `request_handling`,\n`sync_test`); several examples use **`Result`** and structured\n**`FortressError`** matching instead of panics. CI adds **`cargo run\n--example error_handling`** on Linux and **`cargo check`** for the\n**`custom_socket`** example on **`wasm32-unknown-unknown`**.\n> \n> **Docs:** CHANGELOG, migration guide (0.11 upgrade), user guide, and\nwiki describe the security boundary and that graphics must live in the\napplication crate.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n9cd8a39121fc959272b8b314d2b7633dc0776f50. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-02T16:06:08-07:00",
          "tree_id": "be47194f8f95eae916ea4ba3af8cd01775392766",
          "url": "https://github.com/wallstop/fortress-rollback/commit/20caf394a1f157a4cfbe2877f6891f657a3184ad"
        },
        "date": 1785712461785,
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
            "value": 38,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 84,
            "range": "± 1",
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
            "value": 530,
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
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 132,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 170,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 183,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 256,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 370,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 345,
            "range": "± 3",
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
            "value": 732,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 183,
            "range": "± 3",
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
            "value": 257,
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
            "value": 418,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 552,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 649,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 783,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1049,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 500,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 654,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 911,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1174,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 94,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 131,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 410,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 663,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 965,
            "range": "± 20",
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
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 79,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 302,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1389,
            "range": "± 32",
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
          "id": "bdcd24ae97634b26d56c61c5384065029140386e",
          "message": "security: freeze bincode compatibility formats (#282)\n\n## Summary\n\n- pin bincode exactly to 2.0.1 and freeze representative input, state,\nreplay, hot-join, and checksum bytes across the centralized codec\n- protect the immutable suite against rewrites and registration bypasses\nin worktree, index, and pull-request modes\n- replace the stale advisory rationale with a daily, time-bounded\ndisposition that locks version, crates.io source, checksum, and every\nrelevant workspace resolution\n- harden the rewritten release-checkpoint fixture against\nGit-version-sensitive advertised-ref pack negotiation\n\n## Scope\n\nThis is the first bounded entrance gate for #273. It does **not** select\nor deploy a replacement serializer and intentionally does not close the\nissue. Candidate evaluation, malformed-input allocation/recursion\nhardening, performance evidence, and migration remain follow-up\nmilestones.\n\nProgresses #273.\n\n## Validation\n\n- default Nextest: 2,891 passed; 71 skipped\n- hot-join Nextest: 3,148 passed; 72 skipped\n- Python toolchain suite: 2,003 passed\n- release-checkpoint module: 23 passed; rewritten-history case: 25/25\nrepeated\n- strict all-target Clippy with `tokio,json,hot-join`\n- Rust 1.86 all-target check\n- browser WASM and Emscripten hot-join checks\n- cargo-deny advisories/licenses/bans/sources and all canonical\nworkspace locks\n- authoritative 19-gate agent preflight\n- four adversarial passes; final consensus has no actionable findings\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Touches serialization compatibility, exact dependency pinning, and\nsupply-chain enforcement across workspaces; runtime bytes are asserted\nunchanged but any codec or lock drift will fail CI until dispositions\nare updated.\n> \n> **Overview**\n> Pins **bincode** to **`=2.0.1`** and adds a first bounded gate for\nissue #273: freeze representative serialized bytes without changing\non-wire runtime output.\n> \n> **Immutable compatibility suite** — New\n`serialization_golden_bincode_2_0_1` fixtures pin fixed-width inputs,\nrich state, replay envelopes, hot-join payloads, and checksum-derived\nbytes across the centralized codec (protocol `Message` bytes stay on\nexisting wire goldens). The wire-golden immutability hook now blocks\nrewrites of this suite and requires an active `#[cfg(test)]`\nregistration in `lib.rs`; successors must be separately named, not\nedited in place.\n> \n> **Supply-chain policy** — `RUSTSEC-2025-0141` is documented in\n`supply-chain/advisory-dispositions.toml` (owner, **2026-11-02** review\ndate, exit criteria). `scripts/ci/check-advisory-dispositions.py` fails\nif `deny.toml` ignores drift from that file, versions unpinned, lock\nchecksum/source mismatch, or deadlines pass. Enforcement runs in\n**pre-commit**, **ci-security** (daily schedule), and **agent\npreflight**.\n> \n> **Docs / hygiene** — Design decision log entry, changelog note, and a\nrelease-checkpoint test fix (push rewritten history to an empty remote\ninstead of force-pushing over a populated fixture remote).\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n842d9607715f356721463284cd1fb8caf32520cf. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-02T23:31:12Z",
          "tree_id": "5c861318956d0952ab25ab8fb2e9edb30ad04857",
          "url": "https://github.com/wallstop/fortress-rollback/commit/bdcd24ae97634b26d56c61c5384065029140386e"
        },
        "date": 1785714308715,
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
            "range": "± 1",
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
            "value": 38,
            "range": "± 1",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 532,
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 133,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 176,
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
            "value": 259,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 380,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 347,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 478,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 735,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 183,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 213,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 262,
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
            "value": 422,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 557,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 655,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 784,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1055,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 485,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 654,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 934,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1201,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 95,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 131,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 401,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 683,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 947,
            "range": "± 54",
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
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 80,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 301,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1389,
            "range": "± 7",
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
          "id": "ead1837497cf8de075c9024dbb6a6c9f0276db0e",
          "message": "Harden bounded bincode input decoding (#283)\n\n## Summary\n\n- reject trailing bytes inside hot-join state snapshot payloads\n- route replay inputs and coordinated graceful-drop backfill through the\nshared bounded decoder\n- apply the existing recursion-depth guard to custom `Config::Input`\ndeserializers in every build\n- preserve per-value replay limits without capping the whole remaining\nconcatenated replay\n- document the precise Serde-managed allocation boundary\n- harden the pre-existing rewritten-history release fixture against Git\n2.54 missing-object failures\n\nProgresses #273. This PR intentionally does not select a replacement\nserializer, change network/replay bytes, or close the issue.\n\n## Why\n\nA `Copy` result does not prove its custom `Deserialize` implementation\nis non-recursive: it may recursively decode and discard an owned helper\nbefore returning. Replay and coordinated-drop input paths also still\nused generic bincode decode, while hot-join state accepted nested\ntrailing bytes.\n\nCI also reproduced the pre-existing release fixture's\nGit-version-sensitive force-push failure. Publishing the deliberate\nrewrite to a fresh empty bare remote removes advertised-old-ref pack\nnegotiation from that test.\n\n## Validation\n\n- agent preflight: all checks passed, including 286 release tests and\n1,392 links\n- default all-target Nextest: 2,954/2,954 passed (71 skipped)\n- hot-join all-target Nextest: 3,207/3,207 passed (72 skipped)\n- release-checkpoint module: 23/23; rewritten-history case: 25/25\nconsecutive reproductions\n- strict all-target Clippy with `tokio,json,hot-join`\n- 169 doctests passed (54 ignored)\n- Rust 1.86 workspace all-target check\n- `wasm32-unknown-unknown` hot-join check\n- no-default-features recursive custom-input regression\n- formatting and `git diff --check`\n\n## Compatibility\n\nValid wire, replay, state, bridge-input, and checksum bytes are\nunchanged. The new failures apply only to malformed/trailing input or\nvalues exceeding existing Serde-managed allocation/recursion policies.\nHand-written `Deserialize` implementations remain responsible for\nallocations they perform outside the deserializer.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Changes fail-closed behavior on untrusted decode paths (replay, drop\nbackfill, hot-join state) where bugs could affect availability or\nsession handling, but valid formats are unchanged and scope is bounded\ndeserialization hardening rather than auth or persistence.\n> \n> **Overview**\n> Hardens **peer-controlled bincode decoding** so malformed payloads\nfail closed without oversized Serde-managed allocations or unbounded\nrecursion, while **valid wire, replay, and snapshot bytes stay\nunchanged**.\n> \n> **Replay inputs** and **coordinated graceful-drop backfill** now use\nthe same bounded decoder as normal and hot-join bridge inputs (per-value\nallocation cap, recursion-depth limit, exact consumption where\nrequired), replacing generic `decode` on those paths. **Replay** uses a\nnew prefix helper so many concatenated inputs can be decoded from a\nstream larger than the per-value cap without rejecting the whole\nremainder.\n> \n> **Hot-join state snapshots** decode through a depth-limited serde\n**seed** with bincode’s consumed-byte count, so **trailing bytes inside\nthe nested `state_bytes` payload** are rejected—not only at the outer\nmessage boundary.\n> \n> The library drops the assumption that `Copy` on `Config::Input` proves\nnon-recursive decoding; custom `Deserialize` impls that recursively\ndecode helpers are now depth-limited on every affected path. Docs and\nchangelog state that allocations performed **outside** the deserializer\nin hand-written `Deserialize` remain the impl’s responsibility.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nb550b18cf33e729c2ae2c60b2c63a63765393a96. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-02T17:14:32-07:00",
          "tree_id": "77ac7d96ef52e8ba58906aa4d28603ff4c54c009",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ead1837497cf8de075c9024dbb6a6c9f0276db0e"
        },
        "date": 1785716603168,
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
            "value": 84,
            "range": "± 5",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 530,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 28,
            "range": "± 1",
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
            "value": 130,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 175,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 183,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 257,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 378,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 344,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 478,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 731,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 182,
            "range": "± 1",
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
            "value": 260,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 351,
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
            "value": 554,
            "range": "± 30",
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
            "value": 781,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1049,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 486,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 621,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 884,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1136,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 94,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 131,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 405,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 680,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 976,
            "range": "± 21",
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
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 22,
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
            "value": 296,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1368,
            "range": "± 12",
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
          "id": "734c802dbcb5bbfb0930fbf12e278a33ce3233ce",
          "message": "Close remaining hardening issues (#284)\n\n## What\n\nThis single Session 160 closeout PR resolves the repository's remaining\nopen hardening work:\n\n- fixes #278's Windows Miri spectator false failure by giving\nintegration-style spectator tests a frozen protocol clock;\n- records #281's measured `uv` no-go and refreshes all available\nPython/browser dependencies;\n- fixes the Git 2.54 rewritten-history fixture by keeping the prepared\nsource commit explicitly reachable;\n- completes #273 by replacing unmaintained `bincode` 2.0.1 with exact\n`bincode-next` 2.1.0, removing RUSTSEC-2025-0141's exception, banning\nthe original package, and bounding zero-sized replay decode work.\n\n## Compatibility and safety\n\n- The dependency remains available internally as `bincode`, so codec\ncall sites and public APIs are unchanged.\n- Every immutable bincode 2.0.1 network/replay/hot-join/checksum vector\npasses byte-for-byte.\n- `bincode-next` 2.1.0 is the newest release compatible with Rust 1.86;\n3.x requires Rust 1.90.\n- Replay decode rejects more than 1,048,576 cumulative zero-sized inputs\nbefore entering the hostile frame's loop. The fixed ceiling preserves\nexisting `ReplayDecodeConfig` struct literals.\n- Cargo audit is clean; cargo-deny now rejects reintroduction of the\noriginal unmaintained package.\n\n## Performance\n\nPaired Criterion measurements on the same host:\n\n- message roundtrip: about 3.5% faster;\n- input serialization: unchanged;\n- `encode_into`: within noise;\n- trivial decode: about 0.07 ns/call slower (below one CPU cycle), while\noverall roundtrip improved.\n\nAll results are far inside the CI regression gate's 1.50 median-ratio\nthreshold.\n\n## Validation\n\n- default Nextest: 2,900 passed, 71 skipped;\n- hot-join Nextest: 3,154 passed, 72 skipped;\n- focused Miri: zero-sized replay tests 4/4 and immutable vectors 4/4;\n- libFuzzer: 10,000 replay-decode + 10,000 message-parser cases;\n- Rust 1.86, strict Clippy, semver (196 checks), no-default-features,\nbrowser WASM runtime, Emscripten, fuzz, Loom, allocation contract:\npassed;\n- exact MkDocs, Markdown, wiki consistency, 1,389 links, strict rustdoc,\n160 doctests: passed;\n- all-file agent preflight: passed, including 286 release and 66\nCI-toolchain tests.\n\n## Review readiness\n\n- Build/tests: PASS\n- Zero-panic/determinism: PASS\n- Error handling/test breadth: PASS\n- Agent preflight: PASS\n- Design log/CHANGELOG: YES\n\nFixes #273.\nFixes #278.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Serialization and replay decode are contract-critical paths; risk is\nmoderated by immutable golden bytes and a targeted decode ceiling, but\nany serializer regression would affect network, replay, and checksum\ncompatibility.\n> \n> **Overview**\n> Closes remaining hardening work by swapping unmaintained **bincode\n2.0.1** for exact **bincode-next 2.1.0** behind the existing internal\n`bincode` dependency alias. Wire/replay/checksum bytes and public codec\nAPIs stay unchanged (golden vectors still pass); **cargo-deny** bans the\noriginal package and the **RUSTSEC-2025-0141** advisory exception is\nremoved. Dependabot ignores **bincode-next** 3.x until Rust MSRV rises\npast 1.86.\n> \n> **Replay decode** now rejects more than **1,048,576** cumulative\nzero-sized inputs before the per-frame input loop, closing CPU\nwork-amplification that byte and allocation limits cannot bound for\nzero-sized `Config::Input` types. `ReplayDecodeConfig` shape is\nunchanged.\n> \n> **Spectator unit tests** use a frozen protocol clock so Miri/Windows\nruns do not spuriously hit disconnect timeouts during staging loops.\n**ARM64 cross-compile** and **`cargo_linker`** use a global\n**`RUSTFLAGS`** override instead of empty per-target flags so GCC-based\nCI can override incompatible **lld** link args from\n`.cargo/config.toml`.\n> \n> A **release checkpoint** test keeps the prepared commit reachable via\nan explicit ref so Git 2.54 maintenance does not prune objects\nmid-fixture. Docs/wiki/changelog and minor Python/browser doc-tool bumps\nalign with the above.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n168fd27ccf47198b924c5a58b636ea33ea606728. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-02T23:51:07-07:00",
          "tree_id": "601aa6c709c7a220110668cf311c425c372fbba2",
          "url": "https://github.com/wallstop/fortress-rollback/commit/734c802dbcb5bbfb0930fbf12e278a33ce3233ce"
        },
        "date": 1785740385498,
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
            "value": 38,
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
            "value": 38,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 45,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 531,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 25,
            "range": "± 1",
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
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 107,
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
            "value": 172,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 193,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 259,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 380,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 350,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 483,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 734,
            "range": "± 6",
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
            "value": 211,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 260,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 353,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 420,
            "range": "± 6",
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
            "value": 658,
            "range": "± 9",
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
            "value": 1051,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 485,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 661,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 933,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1177,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 95,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 132,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 447,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 704,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 988,
            "range": "± 20",
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 81,
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
            "value": 1356,
            "range": "± 5",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "304587157+wallstop-auto-releaser[bot]@users.noreply.github.com",
            "name": "wallstop-auto-releaser[bot]",
            "username": "wallstop-auto-releaser[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f1d0c47253c981c1da7cf40fe0a27bfc91a3caab",
          "message": "Prepare v0.12.0 release (#285)\n\nAutomated preparation for Fortress Rollback v0.12.0.\n\nCloses #227\n\nAfter this PR is green and merged, run **Release - Publish Crate** on\n`main` with `0.12.0`.\n\nCo-authored-by: github-actions[bot] <github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-03T08:59:57-07:00",
          "tree_id": "5c79a3aa9804f817c529767c644d2f487bdfb29e",
          "url": "https://github.com/wallstop/fortress-rollback/commit/f1d0c47253c981c1da7cf40fe0a27bfc91a3caab"
        },
        "date": 1785773284986,
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
            "value": 34,
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
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 308,
            "range": "± 13",
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
            "value": 62,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 164,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 585,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 98,
            "range": "± 2",
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
            "value": 167,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 189,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 228,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 343,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 294,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 418,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 666,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 185,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 183,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 232,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 290,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 349,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 472,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 545,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 663,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 931,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 465,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 629,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 846,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1036,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 102,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 137,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 434,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 712,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1071,
            "range": "± 37",
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
            "value": 1232,
            "range": "± 7",
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
          "id": "aaaaee44bbbcdd001989b9a5014245039c2a46e7",
          "message": "ci: harden static analysis and network nightly (#288)\n\n## Summary\n\n- close #287 with blocking Ruff, checksum-verified ShellCheck,\nsecurity-extended CodeQL, and pinned cargo-shear across all four Cargo\nworkspaces\n- stabilize the recurring 3/4-peer Network Nightly failures with the\ncorrect exclusive-frame oracle, rollback-aware target snapshots,\nchecksum equality, and a bounded test-only READY/ACK completion barrier\n- incorporate and advance #286's action updates, refresh all compatible\ndependencies and fixture toolchains, and extend Dependabot coverage\n- make Dependabot auto-merge poll every raw transient required-check\nstate while failing closed on malformed GitHub CLI JSON\n\n## Root causes\n\nThe nightly harness treated a 100-frame exclusive range as if frame 100\nwere also required, then let peers exit independently before final\nN-peer gossip converged. The auto-merge helper classified raw\n`IN_PROGRESS` as a terminal failure and could fail open when `jq`\nrejected malformed state values inside a Bash `if` call chain.\n\n## Validation\n\n- formatting, workspace all-target check, and strict Clippy: pass\n- default all-target Nextest: 2,998 passed, 71 skipped\n- hot-join all-target Nextest: 3,252 passed, 72 skipped\n- Python repository suite: 2,063 passed\n- doctests: 168 passed, 54 intentionally ignored\n- exact Network Nightly 3/4-peer zero-retry soak: 20/20 passed across 10\niterations\n- agent preflight `--all --auto-fix`: all groups passed\n- four RustSec audits, cargo-deny, workspace-lock validation, and\ncargo-shear across all four roots: pass\n- Ruff, ShellCheck, actionlint, CodeQL predicate fixtures, shell\nportability, browser WASM, Emscripten, Godot API 4.7 Clippy, and wasm\nbrowser runtime: pass\n- adversarial review loops for network, static analysis, dependencies,\nauto-merge, and the integrated branch: zero remaining findings\n- exact-head hosted CI: 15/15 workflow groups green (83 jobs successful,\n3 policy skips)\n\n## Review readiness\n\n- Build/tests: PASS\n- Zero-panic production scan: PASS (no production Rust changes)\n- Determinism: PASS (retained target-state checksum asserted across\npeers)\n- Agent preflight: PASS\n- Error handling: PASS\n- Tests breadth: PASS\n- Design log reviewed: N/A (test/CI/tooling-only)\n- CHANGELOG reviewed: N/A (no public or production behavior change)\n\nThe authoritative hosted Godot 4.7.1/Emscripten 4.0.20 job built and\nexercised both threaded and non-threaded Chromium exports successfully.\n\nCloses #287.\nSupersedes #286.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Changes affect merge gating, security scanning enforcement, and pinned\nCI action behavior across many workflows; production Rust library code\nis largely untouched, but a misconfigured CodeQL allowlist or auto-merge\nvalidator could block or incorrectly allow merges.\n> \n> **Overview**\n> **Hardens CI and tooling** with blocking static analysis, immutable\nthird-party action pins, a new CodeQL gate, and safer Dependabot\nauto-merge—plus bumps Godot/Emscripten fixture toolchains and lockfiles.\n> \n> **Static analysis:** Adds `ci-codeql.yml` (Actions/Python/Rust,\n`security-extended`, SARIF upload, blocking `jq` enforcement with a\nnarrow allowlist for known `ci-release-state.yml` findings).\n`ci-quality` gains a blocking **Ruff** job (`ruff.toml`), makes\n**cargo-shear** blocking with checksum-verified install and `--locked\n--deny-warnings` over every workspace from `workspace_locks.py`, and\nwidens path filters (`fuzz/**`, `loom-tests/**`, Python under `scripts`\nand `.github`). `ci-lint` adds checksum-pinned **ShellCheck** over all\ntracked `*.sh` files and expands triggers for `scripts/**` and shell\nscripts.\n> \n> **Supply chain / permissions:** Replaces mutable `@v*` / `@master` /\n`@stable` refs on third-party actions with **40-char commit pins**\nrepo-wide; many workflows now declare `permissions: contents: read`.\n**Dependabot** gains weekly groups for devcontainer Docker/features,\ndocs/release pip, and Godot Emscripten npm.\n> \n> **Auto-merge:** `enable-dependabot-automerge.sh` validates `gh pr\nchecks` JSON before classifying states, treats GitHub’s uppercase\n**accepted** vs **transient** states explicitly, and fails closed on\nmalformed responses (with expanded tests).\n> \n> **Fixtures / deps:** Godot browser CI moves to **4.7.1**, Emscripten\n**4.0.20**, `wasm-bindgen` **0.2.127**; `tests/godot-emscripten`\ntolerates the known threaded dlink console warning. Lockfiles and minor\nPython/shell cleanups; `loom-tests` depends on the main crate only as a\n**dev-dependency**. New contract tests in `test_ci_static_analysis.py`\nlock these policies.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n076c00bbc9042bbdae24e7b3683e1dca81f72898. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-08T13:54:18-07:00",
          "tree_id": "7d285f9a90395dd4503e1fda3b3ef4f524ff9c4a",
          "url": "https://github.com/wallstop/fortress-rollback/commit/aaaaee44bbbcdd001989b9a5014245039c2a46e7"
        },
        "date": 1786222921449,
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 31,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 68,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 256,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 109,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 413,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 84,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 111,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 139,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 147,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 192,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 274,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 293,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 368,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 530,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 122,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 149,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 183,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 235,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 269,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 366,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 448,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 534,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 700,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 451,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 640,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 809,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 954,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 95,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 469,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 795,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1220,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 9,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 71,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 243,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 943,
            "range": "± 11",
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
          "id": "d6035ff694fb2e703cff34da62baa14716fa39f3",
          "message": "test(soak): replace flaky RSS oracle with exact live-heap boundedness gate (#298)\n\n## Summary\n\nRepairs the ~25-day `CI - Network Tests (Nightly)` failure streak on\n`main` (21 of 23 daily scheduled runs failing since 2026-07-31) by\nreplacing the soak test's raw-RSS boundedness oracle with an exact\nlive-heap oracle. Also incorporates the open Dependabot action upgrades\n(#296) so the dependency queue stays current.\n\n## Root cause (data-backed)\n\n`four_million_frame_soak_preserves_bounds_replay_and_lifecycle` asserted\nRSS growth < 5% per virtual hour via `/proc/self/statm`. Every one of\nthe last failures shares one signature at `tests/network/soak.rs:350`:\n\n```\nRSS grew by at least 5% in one post-warmup virtual hour: ~7.5 MB -> ~8.7 MB bytes\n```\n\n- Identical code passed and failed on different days: head `04dd933`\npassed 2026-08-17, then failed 2026-08-18 through 2026-08-21; head\n`b46fa91` passed 2026-08-22 with only unrelated dependency bumps\nchanged.\n- Local Linux release reproduction ran all 47 nightly `multi_process`\ntests and two full soaks green.\n- RSS measures kernel residency, not live memory: transparent-huge-page\ndecisions, glibc arena retention, and `MADV_FREE`/reclaim behavior move\nit by megabytes with zero leak. Runner-day conditions flipped the same\ndeterministic workload across the threshold.\n\n## Fix\n\n- Install `stats_alloc::StatsAlloc<System>` as the network test binary's\nglobal allocator (dev-dependency already used by\n`tests/allocation_contract.rs`; lock-free atomics).\n- Replace both RSS growth assertions with one exact oracle: net\nallocated-minus-freed bytes per post-warmup virtual hour via\n`Region::change()`, ceiling 256 KiB. Sampled at the existing checkpoint\ncadence; cross-platform instead of Linux-only.\n- Measured baseline: alternating −36 KB / +48 KB per-hour nets across\nN=2 and N=4 windows with zero trend (~71M alloc ops / ~4.9 GB churn per\nN=4 window netting near zero — exactly the signal class RSS misread).\n- Keep a per-hour informational RSS diagnostic line for future CI\ndebugging.\n- Four unit tests pin the accounting contract, including stats_alloc's\nrealloc-embedding trap (adding its separate `bytes_reallocated` tally\ndouble-counts growth; measured as a phantom +3.2 MB/hour before being\nhand-checked against raw Stats).\n\n## Dependency aggregation\n\nCherry-picks Dependabot's `github-actions-all` group bump\n(`docker/setup-buildx-action` v4 SHA update ×3 sites,\n`taiki-e/install-action` v2.85.10 → v2.86.3), superseding #296.\n\n## Validation\n\nLocal (per operator constraint, CPU-heavy validation moved to CI):\nformatting passes, arithmetic unit tests pass, targeted short soaks\nestablished the measured data above, `actionlint` clean. CI owns strict\nClippy, full matrices, and the hosted nightly soak rerun.\n\nCloses #296 (superseded by direct incorporation).\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Changes how the long-running soak detects leaks (allocator\ninstrumentation and a new growth ceiling) and updates pinned GitHub\nActions SHAs used across CI. Failure modes are test/CI reliability\nrather than production auth or data handling.\n> \n> **Overview**\n> Stops the nightly soak from failing on kernel RSS noise by gating\nboundedness on **net live heap** (`allocated − freed`) instead of\n`/proc/self/statm`.\n> \n> `four_million_frame_soak_preserves_bounds_replay_and_lifecycle` now\ninstruments the network test binary with `stats_alloc` and asserts per\nvirtual hour growth ≤ 256 KiB after warmup. RSS is kept only as a\ndiagnostic `eprintln`. Unit tests pin the realloc accounting so\n`bytes_reallocated` is not double-counted.\n> \n> Also pins `dtolnay/rust-toolchain` jobs with `toolchain: stable`,\nrefreshes `taiki-e/install-action` and `docker/setup-buildx-action`\nSHAs, and loosens the hashed-requirements test to allow multi-hash\nwheels. Small `fill`/`clear`/`is_ok_and` cleanups in session/protocol\ncode.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nd052094fd4b7249514d2d8d18466a47ab673d119. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\n---------\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-23T11:46:25-07:00",
          "tree_id": "1010d7b6124ebb12c507f9bbc70aa4d3899278f8",
          "url": "https://github.com/wallstop/fortress-rollback/commit/d6035ff694fb2e703cff34da62baa14716fa39f3"
        },
        "date": 1787511300459,
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
            "value": 92,
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
            "value": 64,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 167,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 593,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 31,
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
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 99,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 127,
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
            "value": 164,
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
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 294,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 419,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 676,
            "range": "± 1",
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
            "value": 185,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 237,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 290,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 351,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 482,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 546,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 669,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 948,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 463,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 631,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 847,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1037,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 102,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 136,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 435,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 697,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1048,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 21,
            "range": "± 4",
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
            "value": 88,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 347,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1371,
            "range": "± 7",
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
          "id": "2363992a728cdfcb14fb67f2ae3ef9a7f75be1e4",
          "message": "perf: streamline SyncTest and harden mutation CI (#301)\n\n## Summary\n\n- refresh all four Cargo workspace lockfiles through compatible\nminor/patch releases, with `syn` advanced to 3.0.4 and MSRV-blocked\nmajors explicitly left in place\n- add a hermetic pip-vs-uv benchmark, informational quality job,\nartifact, and manual pre-commit hook for #281\n- profile and streamline `SyncTestSession::advance_frame`: prune\nchecksum history once and avoid allocating a mismatch vector on the\nconsistent hot path\n- repair mutation CI after the latest-main timeout: pin cargo-mutants\n27.1.0, use exact-head diff scope for PR/push, dynamically shard the\n4,696-mutant full corpus on Linux, exclude formal-proof-only false\nmutants, and aggregate every shard fail-closed\n- complete the #287 tool-gap audit; track the two real follow-ups\nseparately in #299 (incremental Python typing) and #300\n(standalone-workspace supply-chain coverage)\n\n## Latest-main RCA\n\nThe post-merge main audit found 18 successful workflow runs plus one\ncancelled mutation run. The old workflow serially repeated 5,668 mutants\non Linux, macOS, and Windows; included 972 Kani-only mutants invisible\nto ordinary tests; used derived per-mutant timeouts up to 1,055 seconds;\nfloated cargo-mutants; passed a list-only runtime flag; parsed the wrong\nreport path; and swallowed failures.\n\nThis PR replaces that topology with one authoritative baseline and\nbounded exact-head shards. Diff/filtered runs reject every missed or\ntimed-out mutant. Full sweeps reject every timeout and require at least\nan 80% global mutation score. Missing artifacts, incomplete shard\nindexes/counts, contradictory exit statuses, infrastructure failures,\nand cancelled dependencies all fail the summary.\n\n## Validation\n\n- `cargo fmt --all -- --check`\n- strict workspace Clippy with `tokio,json`\n- default Nextest: 2,999 passed, 71 skipped\n- hot-join Nextest: 3,257 passed, 72 skipped\n- complete Python suite: 2,083 passed\n- mutation workflow/aggregator contracts: 14 passed\n- exact local mutation diff: 25 mutants; 10 caught, 15 unviable, 0\nmissed, 0 timed out\n- full production mutation listing: 4,696; 0 Kani/proof and 0\n`proof_vec` leaks\n- `cargo deny check` and `cargo audit --deny warnings`\n- `python3 scripts/ci/agent-preflight.py --all`\n- actionlint, workspace lock freshness, rustdoc, and doctests\n- final adversarial re-review: zero remaining findings\n\nCloses #281.\nCloses #287.\nAdvances #297; the remaining P2P/spectator sweep stays open.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Changes rollback checksum verification in\n`SyncTestSession::advance_frame` (two-pass logic with documented\nside-effect idempotency) plus CI now gates PRs on mutation results—both\naffect determinism testing and merge policy.\n> \n> **Overview**\n> **SyncTest hot path:** `SyncTestSession::advance_frame` now prunes\nchecksum history **once** per advance (via `prune_checksum_history`)\ninstead of repeating an O(history) `BTreeMap` retain on every frame in\nthe window, and uses an **`any()` probe** so the common all-consistent\ncase avoids building a mismatch `Vec`. Mismatch reporting and rollback\nbehavior are unchanged; new regression tests lock down multi-frame\n`MismatchedChecksum` payloads and pruning bounds.\n> \n> **Mutation CI:** Replaces the cross-platform serial workflow with a\n**baseline + dynamically sharded Linux** pipeline: PR/push runs\n**`--in-diff`** on changed `*.rs`, pins **cargo-mutants 27.1.0**, uses\nper-mutant timeouts and fail-closed aggregation via\n**`aggregate_mutation_results.py`** (zero missed/timeouts on\ndiff/filtered; ≥80% score on full sweeps). **Kani-only** code is\nexcluded from mutation (`proof_vec.rs`, `exclude_re`, `mutants::skip` on\nproof modules).\n> \n> **Tooling / evidence:** Adds a hermetic **pip vs uv** benchmark\nscript, informational quality CI job, manual pre-commit hook, and\ncontract tests; updates code-review guidance on iterator `clone()` vs\ncollection clone; folds min/max lag in simulation fleet tests to avoid\nredundant iterator clones. Lockfiles refresh minor/patch deps.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nce523fc26d521a5a5db61d036516ca654f03059f. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-24T13:35:07-07:00",
          "tree_id": "206ec0183013bf3431ff81d54e8b1fdc1a4a8c7a",
          "url": "https://github.com/wallstop/fortress-rollback/commit/2363992a728cdfcb14fb67f2ae3ef9a7f75be1e4"
        },
        "date": 1787604193082,
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
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 238,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 46,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 111,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 391,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 74,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 95,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 126,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 124,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 170,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 261,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 228,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 316,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 512,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 121,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 143,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 177,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 226,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 271,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 363,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 424,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 526,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 713,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 361,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 486,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 642,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 778,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 81,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 108,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 334,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 514,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 776,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 19,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 68,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 258,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1064,
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
          "id": "0370e8a7dd71e59d6f72d89d14feeebec6f7a382",
          "message": "perf: close the open hardening and usability backlog (#304)\n\n## Summary\n\nThis branch closes the complete paginated open-issue set in\ngameplay-impact order:\n\n- removes measured P2P and spectator hot-path overhead while preserving\nunknown-source diagnostics;\n- replaces fragmented onboarding with one runnable first-session path\nand short route maps;\n- reduces the crates.io archive from 91 to 58 files and from 1,091,787\nto 914,854 compressed bytes;\n- audits all four authoritative Cargo workspaces in hosted\nvulnerability, deny-policy, license, and freshness jobs;\n- type-checks all 58 production Python modules with strict, pinned mypy\nat the Python 3.10 floor.\n\nIt also raises the docs tooling floor to the latest compatible\npymdown-extensions 11.0.2.\n\n## Measured results\n\n- All-local N=2 P2P callgrind: 743,107,916 → 627,307,916 instructions\n(-15.58%).\n- Warm spectator polling: zero allocations for one- and four-host rows.\n- Per-packet connection-status fan-out: zero clone allocations at\nN=2/4/16.\n- Published archive: 36.26% fewer files, 11.57% fewer unpacked bytes,\n16.21% fewer compressed bytes.\n- Strict mypy: 34 initial diagnostics across 15 files → zero issues in\n58 files.\n- Strict pyright comparison: 495 errors on the same scope, so one\nchecker is maintained.\n\n## Verification\n\n- Latest `main` head `2363992`: all 16 workflow groups successful.\n- Strict Clippy: `tokio,json` and `hot-join,tokio,json` matrices pass.\n- Nextest: 2,903 default and 3,161 hot-join tests pass.\n- Rustdoc: 169 pass, 54 configured ignores.\n- Python: 2,098 tests pass; Ruff and strict mypy pass.\n- Package clean-room verification and all 1,953 packaged library tests\npass.\n- All four Cargo locks pass RustSec and cargo-deny policy; all\ncompatible lock resolutions are current.\n- MkDocs strict build, Markdownlint, wiki validation, actionlint, and\nagent preflight pass.\n- Main-thread adversarial review found and fixed the package fixture\nomission, global license-exception noise, cargo-deny config resolution,\npassive contributor prose, and one stale release packaging assertion; no\nfindings remain.\n\n## Dependency disposition\n\nAll compatible Cargo, npm, docs, and release-tooling dependencies are\ncurrent. The only newer Cargo packages exceed the Rust 1.86 MSRV:\nbincode-next 3.1.1 requires Rust 1.90, and serial_test 4.0.1 requires\nRust 1.93.1.\n\nCloses #297\nCloses #299\nCloses #300\nCloses #302\nCloses #303\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Changes core P2P/protocol event handling and crates.io publish\ncontents, though allocation contracts and expanded supply-chain CI\nreduce regression risk.\n> \n> **Overview**\n> This PR tightens **runtime networking**, **consumer packaging**,\n**supply-chain CI**, **Python tooling**, and **onboarding docs** in one\npass.\n> \n> **Performance:** Input events now carry `peer_connect_status` as a\nshared `Arc<[ConnectionStatus]>` so one packet does not clone an\nN-player status vector per player. All-local `P2PSession` polls skip\nendpoint work after socket accounting (unknown-source diagnostics stay).\nSpectator polls reuse constructor-reserved scratch for disconnect\nbatches instead of allocating each idle tick. Allocation contracts and a\nnew `p2p_all_local` benchmark lock in zero- or bounded-allocation\nbehavior on warmed paths.\n> \n> **crates.io:** `Cargo.toml` switches from a long `exclude` list to an\nanchored **`include`** allowlist (sources, licenses, README, one wire\ngolden test) so published tarballs stay small and self-testable; README\nlinks are adjusted for crates.io.\n> \n> **CI / supply chain:** Security and scheduled freshness jobs run in a\n**four-workspace matrix** (root, fuzz, loom, godot-emscripten) with\n`cargo audit`, workspace-scoped `cargo deny`, and **lockfile-scoped\nlicense exceptions** (NCSA/MPL test-only deps). Freshness reports are\nrequired; version lag stays advisory. Quality CI adds **blocking strict\nmypy** (Python 3.10 floor, pinned 2.3.1) plus a matching pre-commit hook\nfor production `scripts/`.\n> \n> **Docs:** README and docs home become short route maps; new **Getting\nStarted** is the single runnable `SyncTestSession` path (MkDocs/wiki nav\nupdated). User guide quick-start defers to that guide.\n> \n> **Tests:** New contracts cover CI security matrices, package payload,\nPython typing gate, and docs onboarding shape.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n579a4098729acf5121631f667076b3d623ca971d. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-24T18:08:57-07:00",
          "tree_id": "c6565a10cf05f80062f2ac98d0fd475db5fa96c8",
          "url": "https://github.com/wallstop/fortress-rollback/commit/0370e8a7dd71e59d6f72d89d14feeebec6f7a382"
        },
        "date": 1787620653883,
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
            "value": 34,
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
            "value": 39,
            "range": "± 1",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 40,
            "range": "± 1",
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
            "value": 63,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 167,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 608,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 31,
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
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 97,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 122,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 163,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 160,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 224,
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
            "value": 294,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 415,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 650,
            "range": "± 9",
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
            "value": 187,
            "range": "± 2",
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
            "value": 292,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 363,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 465,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 540,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 689,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 918,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 458,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 588,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 786,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 974,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 100,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 137,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 427,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 677,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 980,
            "range": "± 15",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1234,
            "range": "± 5",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "304587157+wallstop-auto-releaser[bot]@users.noreply.github.com",
            "name": "wallstop-auto-releaser[bot]",
            "username": "wallstop-auto-releaser[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "9db0f8abd2bfb829f243c6e2fae6ce8b2fce4af5",
          "message": "Prepare v0.13.0 release (#309)\n\nAutomated preparation for Fortress Rollback v0.13.0.\n\nCloses #227\n\nAfter this PR is green and merged, run **Release - Publish Crate** on\n`main` with `0.13.0`.\n\nCo-authored-by: github-actions[bot] <github-actions[bot]@users.noreply.github.com>",
          "timestamp": "2026-08-25T08:56:50-07:00",
          "tree_id": "becff0175ba3c23cb8c27f978102aad4b0831bc9",
          "url": "https://github.com/wallstop/fortress-rollback/commit/9db0f8abd2bfb829f243c6e2fae6ce8b2fce4af5"
        },
        "date": 1787673903334,
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 93,
            "range": "± 1",
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
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 46,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 65,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 167,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 584,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 31,
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
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 99,
            "range": "± 0",
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
            "value": 349,
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
            "value": 419,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 679,
            "range": "± 14",
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
            "value": 185,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 240,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 292,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 354,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 485,
            "range": "± 85",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 548,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 671,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 948,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 463,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 630,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 848,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1006,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 101,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 137,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 427,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 681,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 995,
            "range": "± 13",
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
            "value": 313,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1229,
            "range": "± 8",
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
          "id": "f610fbdcefd2b008bcc099ce6f22157ce65445a2",
          "message": "fix: harden API boundaries and security CI (#314)\n\n## Summary\n\n- add Result-returning spectator constructors that preserve exact\nstartup errors, reject empty/duplicate failover hosts before endpoint\ncreation, and keep legacy Option wrappers source-compatible\n- add Result-returning RLE/compression and JSON telemetry APIs with\nsource-preserving serializer/allocation errors, explicit compatibility\nfallbacks, and byte-identity regressions\n- exercise session-owned Tokio UDP on Linux, macOS, and Windows plus a\nbounded raw-byte transport in a real headless browser, with\nmalformed-packet rejection and failure artifacts\n- advance the development boundary to 0.14.0 so the repository semver\ngate recognizes the deliberate pre-1.0 public error-enum expansion\n- add a pinned rustdoc-JSON census of 2,709 public symbols across 2,703\ntextual paths, including hidden and alias-associated paths, with a\nmachine-checked removal ledger and exact-head CI snapshot gate\n- make legacy `Frame` arithmetic/conversion and deterministic RNG\nboundary inputs total, preserve one-sample probability stream\nconsumption, and add boundary regressions\n- bound the custom transport example, keep all `NonBlockingSocket` paths\nprompt and silent, and expose saturating drop diagnostics\n- restore the recurring Unsafe Code Audit without bundled-Z3 timeout\nrisk; run it on every PR, recompute reports, parse the real cargo-geiger\nroot row, and fail closed on producer, report-writer, partial-output, or\ncache-completeness failures\n- harden devcontainer tool refresh and Cargo target maintenance with\nexact workspace scope, marker/symlink checks, bounded numeric inputs,\nage-check fail-safe behavior, and PR image builds without publishing\n- synchronize API/integration documentation, feature ownership, wiki\ncontracts, active-plan hygiene, and the deterministic fleet-search\nroadmap\n\n## Security CI root cause\n\nThe two latest scheduled Unsafe Code Audit runs raced their 20-minute\njob timeout after cargo-geiger activated the bundled Z3 source build.\nThe replacement feature census excludes only `z3-verification-bundled`,\nretaining ordinary Z3 verification coverage. The report is no longer\nreused across source changes, and executable workflow tests cover\nwarning exits, fatal/partial output, real safety-symbol columns, and\n`tee` failure.\n\n## Verification\n\n- exact-head composite Nextest: 3,213 passed, 72 skipped\n- strict workspace/all-target Clippy with `tokio,json`: passed\n- complete repository Python suite: 2,152 passed\n- local diff-scope mutation corpus: 41 total, 26 caught, 15 unviable,\nzero missed/timeouts; threshold follow-up: 14 total, 12 caught, two\nunviable, zero missed/timeouts\n- full agent preflight: passed (workflow lint, plan/changelog policy,\nversion/lock consistency, links, docs claims, shell/tooling policies)\n- `cargo doc --no-deps --all-features`, `actionlint`, shell\nsyntax/portability, Markdown/wiki checks, and `git diff --check`: passed\n- four Cargo workspace dependency dry-runs: zero selected updates\n- three independent final adversarial reviews at `f375ccf`: zero\nremaining findings\n- hosted validation at `f375ccf`: all 17 workflows passed, including\nUnsafe Code Audit, platform/Miri/verification matrices, and the\nauthoritative 41-mutant gate\n- exact-head `5468610` spectator audit: full Nextest 2,927/2,927 (71\nskipped); `sync-send` spectator 225/225; `hot-join` spectator 229/229;\n210 rustdoc cases; strict all-feature Clippy; browser no-default checks;\n286 preflight Python tests; warning-denied rustdoc/examples/docs/wiki\ngates\n- focused spectator validation/input-buffer mutation slice: two caught,\nfive unviable, zero missed/timeouts; three independent adversarial\naudits found and closed the duplicate-validation\ncomplexity/error-identity risks before commit\n- supported nightly 0.13.0-to-0.14.0 semver comparison: passed; the\nversion bump resolves the prior exact-head hosted rejection of the\ndeliberate exhaustive `InvalidRequestKind` additions\n- exact-head `10906da` #311 local validation: 2,928 tests passed (71\nskipped), 225 rustdoc cases, strict all-feature Clippy, 286 preflight\nPython tests and every repository preflight gate, warning-denied\nrustdoc, immutable serialization plus v2/v1/legacy wire goldens, and a\nlocked crates.io publish dry run\n- focused JSON mutation slice: four caught, ten unviable, zero\nmissed/timeouts; injected JSON/RLE allocation and serializer failures\nremain distinct from valid empty output\n- exact-head `9368453` #312 local validation: native Tokio ownership\nruntime passed; browser fixture compiled and passed strict WASM Clippy\nin forced browser mode; full Nextest 2,928/2,928 (71 skipped), strict\nworkspace Clippy, 286 preflight Python tests, actionlint, workflow\npolicy tests, and the Emscripten dependency boundary passed\n- implementation commit `3461d68` #313 local validation: deterministic\n2,709-symbol/2,703-path census matched in no-default and complete\nproduction-feature profiles; 2,928/2,928 Nextest (71 skipped), strict\ncomposite Clippy, 352 preflight policy/docs tests, 170 focused\nparser/workflow/wiki tests, warning-denied rustdoc, actionlint, locked\npublish dry run, and the supported 0.13.0-to-0.14.0 semver comparison\npassed\n- #313 adversarial review found and closed shared field/method path\naccounting and 1,299 missing associated paths beneath aliases; the final\nledger records 125 root aliases and 1,425 alias-related symbols, while\ntwo unused `__internal` RLE aliases move to the machine-validated\nremoval ledger\n- exact-head `d92fb07` complete Python suite: 2,163/2,163 passed; hosted\ndocs failure was a brittle literal production-script count and is\nreplaced by semantic census inclusion in the strict Python 3.10 typing\ninventory\n\n## Follow-up scope\n\nThis closes the ordered audit children #310 through #313. The parent\n#297 acceptance evidence is reconciled at this exact head and will close\nwith this PR.\n\nCloses #310.\nCloses #311.\nCloses #312.\nCloses #313.\nCloses #297.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **High Risk**\n> Pre-1.0 breaking public API and error-enum expansion affect downstream\nmatches and spectator failover behavior; security CI and new runtime\ngates change what must pass on every PR.\n> \n> **Overview**\n> This PR closes the post-audit public API work by **bumping the crate\nto 0.14.0** and documenting deliberate breaking changes: exhaustive\n`InvalidRequestKind` variants for empty/duplicate spectator failover\nhosts, removal of unused `__internal` RLE aliases, and expanded fallible\nsurfaces while keeping legacy `Option` wrappers.\n> \n> **Session and serialization APIs** gain `Result`-returning spectator\nstartup (`try_start_spectator_session` / `_multi`), fallible JSON on\nmetrics/telemetry types (`try_to_json` with exact-size reservation), and\nmatching `try_encode` paths for compression/RLE. Failover spectators now\nfail before endpoint creation on invalid host lists.\n> \n> **CI and governance** add a pinned **public API census** job\n(`public_api_census.py` + checked TSV snapshots), **Tokio-owned** and\n**headless-browser** session runtime tests with failure artifacts, and a\nrestored **Unsafe Code Audit** on every PR (no bundled-Z3 geiger run,\nfail-closed report parsing). Mutation baseline and `mutants.toml` now\ninclude `hot-join`. Devcontainer work adds user-owned npm AI CLIs,\n`devcontainer-bootstrap.sh` for bounded `target/` cleanup, and PR-only\ndevcontainer image builds without GHCR push.\n> \n> Agent skills gain **plan hygiene** (pre-commit cap on `PLAN.md`) and a\n**fleet-search roadmap** reference; design-decision logs record the\nabove boundaries.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nfbfaae0650026bff8cbbb887b4f1170441231c10. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-27T08:56:56-07:00",
          "tree_id": "7b4754f22a03673158bb12f469e5f6e1c02e0aa9",
          "url": "https://github.com/wallstop/fortress-rollback/commit/f610fbdcefd2b008bcc099ce6f22157ce65445a2"
        },
        "date": 1787846742484,
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
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 68,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 257,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 45,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 110,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 413,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 81,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 101,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 132,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 137,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 180,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 269,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 256,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 345,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 516,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 117,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 137,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 173,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 211,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 258,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 355,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 433,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 524,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 696,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 440,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 634,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 791,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 940,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 94,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 467,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 785,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1192,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 9,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message/encoded_len",
            "value": 1,
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
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 239,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 898,
            "range": "± 7",
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
          "id": "6cdbf0669ed60956527e791b5356dbff3b2b4382",
          "message": "fix: harden timing and serial correctness (#315)\n\n## Summary\n\n- validate synchronization and disconnect timing at every construction\nboundary\n- replace overflow-prone absolute deadlines with saturating elapsed-time\ncomparisons\n- make floor round trips wrap-safe with reserved-zero serial ordering\nand bootstrap spoof protection\n- harden chaos delays, saved-state access, input rollback, and\nsynchronization invariants against panic\n- enforce the production zero-panic contract by banning `debug_assert!`\nunder `src/`\n- close mutation gaps in gap-fill rejection, symmetric jitter,\nready-packet ordering, protocol timer equality, floor serial ordering,\ninvariant detection, and empty drop certificates\n- stop tracking ignored local `progress/` logs\n- synchronize the changelog, API/formal contracts, user guide, public\nAPI census, and wiki mirrors\n\nCloses #297.\n\n## Red/green evidence\n\nRed regressions reproduced:\n\n- zero and sub-millisecond synchronization intervals\n- disconnect notification delay exceeding disconnect timeout\n- `Instant + Duration::MAX` protocol and chaos scheduling\n- floor-round wraparound and a forged maximum bootstrap reply\n- empty saved-state modulo\n- paranoid-build production `debug_assert!` panics\n- surviving mutations in saturated gap-fill rejection, nonzero jitter,\nzero-reorder ready sorting, protocol timer equality, floor serial\nhalf-range ordering, SyncLayer invariant detection, and an empty\ncoordinated-drop certificate\n\nGreen local evidence:\n\n- strict production Clippy with panic, unwrap, expect, unreachable,\ntodo, unimplemented, and indexing lints denied\n- final baseline Nextest: 2,941 passed, 71 skipped\n- final all-feature workspace Nextest: 3,392 passed, 72 skipped;\nall-feature Clippy passed with warnings denied\n- earlier full hot-join Nextest: 3,196 passed, 72 skipped\n- rustdoc: 169 passed, 54 ignored\n- pinned Miri seeds 0–2 for maximum timers, floor wrap, chaos delay, and\nempty saved states\n- Kani synchronization counter and bounds proofs\n- complete agent preflight; 38 Safety workflow tests; actionlint; API\ncensus; Markdown, link, and wiki checks\n- `cargo audit`, `cargo deny check`, and all four lockfile dry-run\nupdates\n- configuration mutation slice: 11 caught, 3 unviable, 0 missed/timeouts\n- iterated 27-mutant input/chaos/saved-state boundary corpus with no\nremaining scored survivor\n- exact hosted mutation follow-up: 5 caught, 1 structurally unviable, 0\nmissed/timeouts across invariant, serial, timer, and coordinated-drop\ntargets\n- subsequent hosted aggregate findings (interruption/shutdown equality\nand invariant-checker execution): all 4 caught in the exact focused\nlocal rerun\n- release-profile review finding fixed: diagnostic-only invariant test\nis gated to debug/paranoid; overflow-check and warnings-as-errors\nrelease reproductions pass\n- final hosted diff corpus contains 151 production mutants for\nauthoritative classification\n- zero tracked `progress/**` paths; ignored progress files remain\navailable locally\n\n## Mutation scope\n\nThe `sync-send` and non-`sync-send` `ChaosSocket` trait implementations\nare mutually exclusive. Their wrappers now delegate directly to shared,\nmutation-tested inherent methods, and the mutation configuration\nexcludes only those trivial wrappers so the uncompiled copy cannot\ncreate a guaranteed survivor.\n\n## Dependency and main-branch audit\n\nAll 17 workflows on base commit `f610fbd` completed successfully.\nDry-run updates selected zero compatible lockfile changes. bincode-next\n3.1.1 and serial_test 4.0.1 exceed the project's Rust 1.86 MSRV, so the\nlatest compatible releases remain selected.\n\n## Review notes\n\nThe main-thread adversarial loop found and fixed a bootstrap\nmaximum-round bypass plus two fail-open Safety CI scan paths.\nPost-publication mutation passes found and fixed the boundary and\ninvalid-state test gaps above plus one feature-gating blind spot. The\nfinal frozen diff had no remaining concrete findings.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **High Risk**\n> Touches core UDP protocol timing, floor-gossip ordering across serial\nwrap, and session startup validation—areas that affect sync, disconnect\nbehavior, and relay confirmation if regressions slip through.\n> \n> **Overview**\n> This PR hardens **network timing, floor-gossip serials, and failure\nmodes** so debug/paranoid builds and extreme configs cannot panic or\nstall sessions.\n> \n> **Configuration** adds public **`SyncConfig::validate`** plus\n**`validate_disconnect_timing`**, wired into session builders and\n**`UdpProtocol::new`**. Invalid sync roundtrip counts, sub-millisecond\nintervals, and a notify delay greater than disconnect timeout now fail\nwith structured errors before endpoints are created.\n> \n> **Protocol timers** stop using absolute `Instant` deadlines\n(`shutdown_timeout`, keepalive, disconnect, sync retry). They compare\n**elapsed durations** with saturating math, use **strictly-after**\ninterval boundaries in poll logic, and guard sync roundtrip decrements\nwith **`checked_sub`**.\n> \n> **Floor-round relay** replaces plain numeric `round_seq` ordering with\n**half-range wrapping comparison**, **reserved serial 0**, and bootstrap\nprotection against forged high serials—so post-`u32::MAX` replies stay\nvalid and stale pre-wrap packets are dropped. Invariant checks\n**`report_violation`** instead of **`debug_assert!`**.\n> \n> **`ChaosSocket`** schedules delivery from **enqueue time + delay**\n(not `deliver_at`), saturates stats, shares **`send_to_impl`** between\nfeature-gated trait impls, and extends **cargo-mutants** excludes for\nthe thin wrapper methods.\n> \n> **Input queue, sync layer, metrics, saved states, and coordinated\ndrop** replace remaining production **`debug_assert!`** paths with\ntelemetry or fail-closed returns; gap-fill now fails when a saturated\ninsert looks successful at **`i32::MAX`**. **Safety CI** fails if any\n**`debug_assert`** variant appears under **`src/`**.\n> \n> Docs/changelog/API census/wiki mirror the new validation and serial\nsemantics; several **`progress/`** session logs are removed from the\ntree.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nfa8b771d9a2993fef5f36e256bf2ae220fc9b7e8. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-27T15:04:38-07:00",
          "tree_id": "e36487a931f33c26b75337560eb4554ce5264e4d",
          "url": "https://github.com/wallstop/fortress-rollback/commit/6cdbf0669ed60956527e791b5356dbff3b2b4382"
        },
        "date": 1787868806179,
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
            "value": 83,
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
            "value": 56,
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
            "value": 451,
            "range": "± 2",
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
            "value": 36,
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
            "value": 129,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 175,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 293,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 256,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 481,
            "range": "± 67",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 349,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 475,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 698,
            "range": "± 9",
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
            "value": 357,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 418,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 550,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 653,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 781,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1039,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 511,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 635,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 879,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1079,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 93,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 131,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 394,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 665,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 954,
            "range": "± 23",
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
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 312,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1452,
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
          "id": "b621ffbd6f42147511ec36e6195f96f91f0f9c0e",
          "message": "feat: publish deterministic sometimes-state census (#318)\n\n## Summary\n\n- add five fixed-order, monotonic production observations and include\ntheir final per-peer/aggregate vectors in deterministic simulation trace\nidentity\n- publish strictly validated, atomically written, domain-separated\norganic and targeted census artifacts from the unchanged nightly fleet\n- add real production-path positives, adjacent negatives, replay/zero\ncontrols, mutation-sensitive artifact tests, and the merge/upload\nworkflow\n- refresh all currently compatible Cargo/tool/action pins, including\nRust nightly 2026-08-26, Codex, Ruff, Vale, and CI actions\n\n## Acceptance evidence\n\n- final adversarial review: zero Critical/High/Medium/Low findings\n- full workspace Clippy with `tokio,json`: pass\n- workspace/all-target Nextest: 3,082 passed, 72 skipped\n- `cargo c`: pass; `cargo t`: 2,975 passed, 72 skipped\n- pinned two-seed Miri matrix: 3,306 passed, 10 ignored\n- workflow/static tests: 179 passed; Actionlint and all four audit/deny\nmatrices pass\n- public API census: 2,716 symbols match\n- exact nightly seed-316 run: 1,000 schedules / 5,000,000 steps; nine\nJSON artifacts totaling 9,154 bytes\n- warmed interleaved B/T ratios: 101.9491%, 101.1156%, 103.2193%; median\nratio 101.6623% (limits: 115% paired, 110% median)\n\nThe initial nightly intentionally reports organic zeroes without\nfailing. A future seven-window ratchet remains a separately approved\ndecision.\n\nCloses #316\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Touches P2P confirmation folding and rollback observation hooks on hot\npaths, but bits are observation-only; larger risk is CI/nightly census\nworkflow and toolchain bumps affecting reproducibility and security\ngates.\n> \n> **Overview**\n> Adds **issue #316** observation plumbing: five fixed-order, monotonic\n“sometimes-state” bits are recorded on the production P2P path (rollback\ndepth at prediction limit, relay lower-floor consumption, connect-status\nnudge queued, sparse earlier rollback checkpoint, input ring one slot\nbelow capacity) and exposed via doc-hidden\n`P2PSession::diagnostic_sometimes_state_vector` without changing wire,\nRNG, or replay semantics.\n> \n> The simulation harness **schema 20** folds per-peer and aggregated\nvectors into deterministic trace identity; nightly fleet runs write\neight organic shards plus targeted probe evidence under\n`FORTRESS_SIM_CENSUS_DIR`, then CI merges, validates, and uploads\npublished census artifacts (separate from failure dumps).\n> \n> **CI/tooling** moves the pinned Rust nightly to **2026-08-26**, bumps\nVale, Ruff, typos, `actions/cache` v6, `taiki-e/install-action`,\ncheckout patches, and adds a `cargo deny` license-check flag-order\ncompatibility branch with workflow contract tests.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n6545a6ba6b549d4dd1a452119b787e71f7acb62c. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-27T23:31:35-07:00",
          "tree_id": "c52f430ce89e97ef04d27415ed4bd03159d39cad",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b621ffbd6f42147511ec36e6195f96f91f0f9c0e"
        },
        "date": 1787899208948,
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
            "value": 34,
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
            "value": 92,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 308,
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
            "value": 41,
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
            "value": 140,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 496,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 31,
            "range": "± 1",
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
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 33,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 96,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 122,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 162,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 160,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 224,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 337,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 297,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 416,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 651,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 158,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 186,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 230,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 290,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 364,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 464,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 540,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 699,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 918,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 460,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 628,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 830,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 1004,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 101,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 136,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 438,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 675,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 971,
            "range": "± 18",
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
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 93,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 352,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1459,
            "range": "± 4",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}