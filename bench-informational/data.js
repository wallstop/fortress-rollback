window.BENCHMARK_DATA = {
  "lastUpdate": 1795240926348,
  "repoUrl": "https://github.com/Ambiguous-Interactive/fortress-rollback",
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
          "id": "e6ad6c17ccfbdfdbd8a726444920a203ee7a857b",
          "message": "test(simulation): bound PCT interleaving and fix a lagging-survivor freeze false positive (#421)\n\n## Summary\n\n- Adds an opt-in, bounded PCT peer-interleaving axis to the\ndeterministic simulation harness (`RunOptions::pct`, at most 1,024\npriority assignments). The historical forward order is unchanged when\nunset, and `pct` is mutually exclusive with differential mode.\n- Uses the axis to run a matched equal-compute comparison against the\ncurrent forward-order scheduler on the nightly schedule generator.\n- Fixes a freeze-frame oracle false positive exposed by the comparison:\na survivor whose confirmed prefix ends *before* another survivor's\nobserved freeze frame was treated as an authoritative `None`\ndisagreement.\n- Records the PCT rejection in the fleet-search roadmap and narrows\n`PLAN.md` to the remaining prefix-replay and LDFI-lite comparison\n(scoped by #420).\n\n## Measured comparison\n\nMatched arms: run id `202611140101`, shard 0, 128 schedules, 640,000\nscheduled steps each.\n\n| Arm | unique organic vectors | unique peer-count buckets | unique\nbehavior signatures | failures |\n| --- | ---: | ---: | ---: | ---: |\n| Forward (baseline) | 6 | 13 | 107 | 0 |\n| PCT, K=16 | 6 | 13 | 107 | 0 |\n\nBoth arms observed exactly `{0, 2, 4, 6, 17, 21}` (5-bit OR of the\nregistry; sparse-earlier remains unreachable organically). Every trace\nhash changed, proving the interleaving was applied. No coverage or\ndefect-discovery gain, so PCT stays out of the nightly fleet; the opt-in\naxis is retained for order-sensitivity regressions.\n\n## Bug found and fixed\n\nA 1-change PCT run on the checked-in D14 lossy graceful-remove schedule\nproduced:\n\n```text\nFreezeFrameDivergence { slot: 4, peer: 3, first_author: 0, expected: None,\n  actual: Some(FreezePoint { frame: 302, ... }) }\n```\n\nThe step-302 visit order left peer 0 at confirmed frame 301 while peer 3\nreached 303 and observed the slot-4 freeze at 302. The oracle treated\npeer 0's not-yet-observed `None` as a disagreement. A survivor that has\nnot reached the freeze frame cannot disagree; a survivor that did reach\nit and never observed a stable `Disconnected` run still fails. Pinned\nby:\n\n- `oracle_ignores_lagging_survivor_before_the_freeze_frame` (unit).\n- `pct_lagging_survivor_does_not_fake_a_freeze_divergence` (D14\nintegration + exact replay).\n- `oracle_ignores_all_survivors_lagging_behind_the_removal_cut`\n(all-`None` case).\n\nAdversarial review follow-up also clarified that PCT shrinking preserves\nthe stable failure class rather than the exact original trace, and\nrefreshed the stale artifact-schema message after the v9 bump.\n\n## Verification\n\n- `cargo nextest run --profile ci --test simulation --features hot-join`\n→ 473 passed, 27 skipped.\n- `cargo nextest run --profile ci --features tokio,json` → 3,184 passed,\n75 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings` → clean.\n- `cargo clippy --tests --test simulation --features hot-join -- -D\nwarnings` → clean.\n- `python3 scripts/ci/agent-preflight.py --auto-fix` → all checks\npassed.\n- Plan hygiene, Agent Skill validation/quality, markdownlint, link\ncheck, typos → green.\n\n## Issue notes\n\n- #403 remains calendar-bound (2/7 scheduled post-redesign windows; five\nremain through 2026-11-19).\n- #420 records the entrance gate and acceptance criteria for\nprefix-replay branching and LDFI-lite.\n- No production behavior, wire format, replay format, or public API\nchanges. No changelog entry is required for test-only work.",
          "timestamp": "2026-11-14T15:26:14-08:00",
          "tree_id": "321a75ff934b20387dba9e66bddac1c28811f741",
          "url": "https://github.com/wallstop/fortress-rollback/commit/e6ad6c17ccfbdfdbd8a726444920a203ee7a857b"
        },
        "date": 1794699258363,
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
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 82,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 274,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 41,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 56,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 156,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 526,
            "range": "± 2",
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
            "value": 100,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 125,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 162,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 176,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 249,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 356,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 340,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 469,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 716,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 175,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 205,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 267,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 344,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 411,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 531,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 643,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 771,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1014,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 381,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 521,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 717,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 936,
            "range": "± 20",
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
            "value": 133,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 402,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 656,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 921,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 36,
            "range": "± 2",
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
            "value": 83,
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
            "value": 1432,
            "range": "± 3",
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
        "date": 1794716871002,
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
            "value": 17,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 19,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 50,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 204,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 102,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 354,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 17,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 49,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 59,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 78,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 87,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 119,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 166,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 175,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 233,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 345,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 104,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 116,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 131,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 171,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 216,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 266,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 348,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 396,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 474,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 233,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 262,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 350,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 455,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 57,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 77,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 362,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 592,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 887,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 15,
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
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 160,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 829,
            "range": "± 89",
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
        "date": 1794728989961,
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
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 67,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 229,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 27,
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
            "value": 48,
            "range": "± 1",
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
            "value": 452,
            "range": "± 3",
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 93,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 125,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 123,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 174,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 267,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 224,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 320,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 500,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 118,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 138,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 178,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 231,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 279,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 377,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 416,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 513,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 700,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 277,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 371,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 534,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 692,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 81,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 111,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 337,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 542,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 761,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 38,
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
            "value": 20,
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
            "value": 273,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1074,
            "range": "± 3",
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
        "date": 1794775160855,
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
            "value": 24,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 85,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 105,
            "range": "± 6",
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
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 68,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 74,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 99,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 87,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 132,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 203,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 153,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 235,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 374,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 113,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 127,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 145,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 184,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 262,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 266,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 337,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 518,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 258,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 323,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 464,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 584,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 111,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 345,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 568,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 771,
            "range": "± 62",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 49,
            "range": "± 13",
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 70,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 262,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1031,
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
          "distinct": false,
          "id": "fae9d40717215859af87db577f24f80227ea493e",
          "message": "perf: trim steady-state per-frame folds and allocations; fix the Kani ProofVec iteration (#428)\n\n## Summary\n\nThree bounded hot-path improvements plus a merge-queue CI fix, with\nred/green evidence for each.\n\n### 1. Main is red: Kani could not compile\n`SyncLayer::oldest_retained_input_frame`\n\nPR #427 added `for queue in &self.input_queues`. `ProofVec<T>` is a real\n`Vec<T>` in every production build but the stack-backed `InlineVec`\nunder `cfg(kani)`, which intentionally exposes only\n`push/get/get_mut/len/first/clone/iter/iter_mut`. The merge-group lanes\nwere the first place `cfg(kani)` compiled, and all Kani tiers failed at\n`src/sync_layer/mod.rs:412` with `&InlineVec<...> is not an iterator`.\n\nFix: iterate through the documented common subset\n(`self.input_queues.iter()`). Swept every `ProofVec` field for the same\npattern; this was the only by-reference iteration.\n\nLocal proof evidence (Kani 0.68 bundle; the local 0.67 bundle is empty,\nso `--cap-lints allow` suppresses a nightly-2026-08-21-only const-assert\nlint that CI's pinned 0.67 toolchain does not emit):\n\n```text\ncargo kani --harness proof_frame_new_valid                      -> 0 of 3 failed, SUCCESSFUL\ncargo kani --harness proof_minimal_sync_layer_initial_state_valid_2p -> 0 of 448 failed (4 unreachable), SUCCESSFUL\n```\n\n### 2. `confirmed_frame` recomputed the relay-topology gate per slot\n\n`P2PSession::confirmed_frame` computed the slot-independent\n`pessimistic_floor_relay_topology` once, then each per-slot\n`remote_slot_confirmed_bound` recomputed it inside\n`remote_slot_confirmed_bound_observation`, rescanning every remote\nendpoint per slot. The gate is now passed in; the wrapper keeps the\nexisting call shape for tests/diagnostics. Pure-predicate reuse, no\nbehavior change.\n\n`cargo bench --bench h16p_mesh -- --quick`, same host and saved\ncriterion baseline:\n\n| N | Before | After | Delta |\n| ---: | ---: | ---: | ---: |\n| 2 | 23.918 ns | 14.980 ns | -37% |\n| 4 | 82.668 ns | 58.861 ns | -29% |\n| 8 | 329.39 ns | 218.36 ns | -34% |\n| 16 | 1.8350 us | 739.11 ns | -60% |\n\n### 3. Idle receive polls reserved a four-message batch\n\n`receive_all_messages_from` (`UdpNonBlockingSocket`, `TokioUdpSocket`)\nreserved four slots before attempting any receive, so an idle poll\nallocated a 704-byte buffer per session per frame. The batch is now\nsized on the first decoded message (four slots then, one per later\nmessage). New allocation contract:\n\n```text\nred:   idle built-in UDP receive poll: allocations: 1, bytes_allocated: 704\ngreen: idle built-in UDP receive poll: 0 allocations\n```\n\n### 4. Spectator serving rebuilt a handle-keyed map every frame\n\n`send_confirmed_inputs_to_spectators` built a `BTreeMap<PlayerHandle,\nPlayerInput<_>>` per confirmed frame per send, and\n`SyncLayer::confirmed_inputs` collected into a heap `Vec`. It now\nreturns a fallibly reserved `SmallVec<[PlayerInput<_>; 4]>` and\n`InputSource` has an indexed-slice implementation documented for the\nascending complete vector `confirmed_inputs` produces. Wire encoding\norder is unchanged.\n\nNew allocation contract (host + one spectator, steady ack polls):\n\n```text\nred:   6 allocation operations / 290 bytes per measured frame\ngreen: 4 allocation operations / 66 bytes, stable across repetitions\n```\n\nThe remaining allocations are the Input message's own compression/queue\nbuffers.\n\nThe checked-in violation-site inventory was regenerated: the removed\nreceive-batch reservation site is gone and the new confirmed-input\nreserve site is registered (`284` entries still match production).\n\n## Verification\n\n- `cargo fmt --check` - clean.\n- `cargo clippy --workspace --all-targets --features\ntokio,json,hot-join` - clean (only the pre-existing `wasm-browser-smoke`\nmanifest warning).\n- `cargo tfull` - **3203 passed, 75 skipped**.\n- `cargo nextest run --features tokio,json,hot-join -E 'test(spectator)'\n--workspace` - 243 passed.\n- `cargo nextest run -p fortress-rollback --lib p2p_session --features\nhot-join` - 253 passed.\n- `cargo nextest run --test allocation_contract --features tokio,json` -\npass.\n- Fresh-seed nightly shard 0 (`FORTRESS_SIM_SEED_BASE=20261115`, 375\nschedules / 1,875,000 steps) - pass, zero oracle failures.\n- Kani tier-1 and tier-2 harnesses above - pass.\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix` - **All\nagent preflight checks passed**.\n- `python3 scripts/hooks/check-changelog-unreleased.py`, markdownlint -\npass.\n\nNo public API, wire format, or behavioral change; the only\nsource-visible change is the internal `SyncLayer::confirmed_inputs`\nreturn type and the internal `InputSource` implementation.",
          "timestamp": "2026-11-15T21:14:35Z",
          "tree_id": "dbbc3651d7fb654854e1af3476f516490f720a19",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/fae9d40717215859af87db577f24f80227ea493e"
        },
        "date": 1794777870149,
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
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 71,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 17,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 91,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 17,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 35,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 52,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 56,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 81,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 110,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 115,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 160,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 215,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 54,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 62,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 106,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 127,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 188,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 210,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 247,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 311,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 158,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 197,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 279,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 359,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 53,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 74,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 329,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 550,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 824,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 14,
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
            "value": 10,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 105,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 398,
            "range": "± 1",
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
        "date": 1794779888523,
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
            "value": 24,
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
            "value": 24,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
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
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 124,
            "range": "± 3",
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 78,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 95,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 131,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 142,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 200,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 292,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 251,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 354,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 545,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 142,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 185,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 229,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 256,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 311,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 392,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 475,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 574,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 759,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 324,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 448,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 627,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 808,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 95,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 133,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 400,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 643,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 935,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 34,
            "range": "± 2",
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 60,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 219,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 868,
            "range": "± 3",
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
        "date": 1794787174247,
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
            "value": 24,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
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
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 123,
            "range": "± 8",
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
            "value": 79,
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
            "value": 139,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 191,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 292,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 253,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 356,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 547,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 142,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 155,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 194,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 255,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 310,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 409,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 474,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 573,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 761,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 322,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 441,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 633,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 815,
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
            "value": 131,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 392,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 668,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 922,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 36,
            "range": "± 2",
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 59,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 214,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 847,
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
          "id": "4e6c2e534778e53b538c743bf1f812e822f3506c",
          "message": "fix: bound hot-join pauses, truncate ended spectators, trim receive staging (#435)\n\n## Summary\n\nCloses #433, #434, and #432.\n\n### Spectator terminal truncation (#434)\n\n`FortressError::SpectatorStreamEnded { frame }` is the explicit terminal\nstate for a spectator whose\nlast host disconnected before the next frame's input arrived. The\nspectator still drains every frame\nreceived before the `Goodbye`; a final input arriving in a later poll\nfrom the removed host is\nrejected as an unknown source. Exhaustive error matches need the new arm\n(breaking).\nRed/green: new\n`spectator_late_final_input_after_host_removal_reports_terminal_truncation`;\nthe two\nall-hosts-drained tests now assert `SpectatorStreamEnded` at the exact\ntruncation frame.\n\n### Hot-join pause bound (#433)\n\nEach timed-out serve records a per-handle retry floor. The next serve\nmay open only after the host\nhas advanced `1, 2, 4, ...` frames (cap 256), so a peer that syncs but\nnever acks cannot keep the\nhost paused in a near-continuous duty cycle. The backoff is shared by\nthe 2-peer and N-peer serve\npaths, cleared on successful join or a fresh endpoint era, and only\nduring the backoff window are\nsimulation inputs withheld from the abandoned reserved endpoint\n(preventing its `pending_output`\noverflow from resetting the budget). Normal reserved-slot input traffic\nis unchanged.\nRed/green:\n`hot_join_timed_out_serves_are_bounded_by_exponential_backoff` drives 12\nforced timeout\nwindows and asserts exact `1,2,4,...,256` progress between serves,\nblocked immediate re-requests,\nand a single-window send-queue high-water bound.\n\n### Receive staging (#432)\n\n- Decoded input frames and per-player input vectors use `SmallVec`\ninline storage for the common\n  `<=4` case (fallible heap growth above it).\n- Consecutive packets reuse the connect-status `Arc<[ConnectionStatus]>`\nwhile gossip is unchanged.\n- `P2PSession::poll_remote_clients` retains its event-staging queue\nacross polls.\n- Warmed one-Input receive poll measured: **8 -> 6 allocation ops** and\n**1408 -> 1152 bytes** at\nN=2 (N=4 1424->1168, N=16 1520->1264). `tests/allocation_contract.rs`\nnow enforces 6 ops / 1280\n  bytes and tightens the send byte ceilings to 96/112/500.\n- The suggested `confirmed_frame` gating was implemented and measured,\nthen rejected: it shifted the\n`4p-loss15-rtt200-4b` deterministic sweep baseline from 7119.4 to 7500.7\nbytes/player/sec for a\n  sub-microsecond fold saving. The sweep is green after reverting it.\n\n## Verification\n\n```text\ncargo fmt --all -- --check                         clean\ncargo clippy --workspace --all-targets --features tokio,json,hot-join -- -D warnings  clean\ncargo nextest run --workspace --features tokio,json,hot-join --no-fail-fast            3504 passed, 76 skipped\ncargo tfull                                                                            3214 passed, 75 skipped\ncargo nextest run --test sessions -E 'test(hot_join)' --features tokio,json,hot-join   43 passed\ncargo nextest run --test allocation_contract --features tokio,json                     pass\npython3 scripts/ci/agent-preflight.py --full --auto-fix                                all checks passed\npython3 scripts/hooks/check-changelog-unreleased.py                                    pass\npython3 scripts/hooks/check-plan-hygiene.py                                            pass\n```\n\nChangelog entries cover the breaking error variant, the hot-join\navailability fix, and the receive\nallocation reduction. `PLAN.md` keeps only the measured-rejected\nfold/send-encode follow-up.\n\n\n### Send-side scratch reuse (#432 continuation, `f673d6c`)\n\n`compression::try_delta_encode_into` backs a doc-hidden\n`try_encode_with_scratch`; `UdpProtocol`\nnow owns the XOR-delta scratch used by both pending-input sends and\nconnect-status nudges. Only the\nqueued RLE output allocates per packet. A new test proves the scratch\npath is byte-identical to\n`try_encode` and does not reallocate once warm. Warmed networked send\nallocation contract:\nN=2 **4 ops/66 B -> 3/58**, N=4 **4/90 -> 3/78**, N=16 **5/482 ->\n4/422**; ceilings tightened to\n3/64, 3/84, 4/440. `sweep_pr_gate` remains green, confirming unchanged\nwire traffic. `PLAN.md` now\ntracks only the measured-rejected confirmation-fold sharing follow-up.\n\n\n### Confirmation-fold sharing (#432 completion, `2dfb7fe`)\n\n`poll_remote_clients` already computes the mesh `confirmed_frame` fold\nfor the gossip-liveness\nnudge. `advance_frame` now reuses that exact value when no endpoint\nevent or disconnect propagation\nchanged a folding input; event handling, live coordinated-drop state,\nand hot-join orchestration\ninvalidate the cache, and `update_player_disconnects` reports whether it\nmutated status. A test-only\nfold counter plus\n`advance_frame_reuses_the_poll_confirmation_fold_when_no_state_changed`\nproves an\nidle advance performs **one mesh fold instead of two**. `sweep_pr_gate`\nremains byte-identical.\n`PLAN.md` is now empty.",
          "timestamp": "2026-11-16T03:24:30Z",
          "tree_id": "435a67dc69b3154f427229b35cd73b476ace902f",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/4e6c2e534778e53b538c743bf1f812e822f3506c"
        },
        "date": 1794800198376,
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
            "value": 24,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 85,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 109,
            "range": "± 4",
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
            "value": 22,
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
            "value": 54,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 71,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 95,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 87,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 131,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 196,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 151,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 235,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 363,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 96,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 111,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 139,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 146,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 183,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 252,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 267,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 340,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 514,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 229,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 315,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 477,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 584,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 82,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 110,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 331,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 547,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 751,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 31,
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
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 48,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 171,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 642,
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
          "id": "41e7476e3db03f54d89237de00a610fa2a5697a4",
          "message": "fix: keep confirmation folds fresh across floor-round prunes; finish #435 review follow-up (#436)\n\n## Summary\n\nFollow-up to #435 (merged as `4e6c2e5`) that closes out its last review\nthread and fixes two\ncorrectness/availability edges found by an adversarial read of the new\nhot-path code.\n\n### Confirmation fold freshness (`drive_floor_round`)\n\n`poll_remote_clients` caches its mesh `confirmed_frame` fold for\n`advance_frame` reuse. A prune is\ndetected in `drive_floor_round`, which runs after that fold and resets\nevery relay's floor-round\nfreshness. In the relay topology a connected relay's contribution is\ncapped at the previously\nconfirmed frame while the round is incomplete, so the cached pre-reset\nfold can exceed the\npost-reset fold. Reusing it confirmed past the re-armed hold — the exact\nconnected-relay reorder\nhazard the round exists to prevent.\n\n`drive_floor_round` now drops the cached fold on a prune in the relay\ntopology. Outside that\ntopology the fold never consumes round-floor state, so steady-state\nreuse is unchanged (including\nsteady relay meshes with no prune).\n\nRed/green:\n-\n`advance_frame_recomputes_confirmation_when_the_prune_rearms_the_floor_round`\n(session-level): red prints `before=Frame(50) after=Frame(100)`, green\nholds at 50.\n-\n`drive_floor_round_drops_the_cached_fold_only_in_the_pruned_relay_topology`\npins both arms\nunder every feature set (verified red under `--features hot-join` too).\n\n### Spectator terminal state with catch-up disabled\n\nWith `catchup_speed == 0` and the spectator behind, `advance_frame`\nattempts no advance and the\nall-hosts-drained terminal check was gated on `frames_to_advance > 0`.\nAfter the last host left,\nthat configuration returned `Ok(empty)` forever. Once no host remains,\nthe terminal drain now\nadvances one buffered frame per call regardless of `catchup_speed` (so\nthe documented \"drain the\nbuffered prefix first\" contract holds for every configuration), then\nreports\n`SpectatorStreamEnded`. While a host remains, zero catch-up still\nattempts no advance.\n\nRed/green:\n`spectator_all_hosts_dropped_with_catchup_disabled_drains_then_reports_terminal_truncation`\n(red without the drain override: `SpectatorStreamEnded { frame: 0 }`\nbefore delivering buffered\nframes; green: frames 0-2 drain, then the terminal error at frame 3).\nThe buffered-drain,\nlate-final-input, and stream-delay terminal tests remain green. Rustdoc,\nchangelog, API contract,\nuser guide, and wiki mirrors describe the drain and the remaining\nhost-present zero-speed\nbehavior.\n\n### Status-snapshot reuse test (#435 review thread)\n\n`on_input_reuses_status_snapshot_across_unchanged_packets` exercises the\npacket path: two packets\ncarrying new frames with identical status arrays share one snapshot\n(`Arc::ptr_eq`), and a changed\n`last_frame` allocates a new snapshot without mutating the earlier\nevent's contents. Forcing the\ncache-compare branch off makes it fail.\n\n### Mutation survivors from main's post-#435 diff run\n\nThe post-merge diff-scope mutation run on `4e6c2e5` reported nine\nsurvivors in that PR's new\ncode. This PR kills them without changing behavior:\n\n- `coordinated_drop_may_change_confirmation` extracts the poll-time\ncache-invalidation predicate\n(one assertion per live drop state) so each `||`/`!` arm is\nindependently pinned.\n-\n`update_player_disconnects_reports_no_change_when_already_at_the_fence_cut`\npins the new\nmutation-return value: a receipt already at the committed fence cut\nreports `false`; a lower\n  receipt adopts the cut and reports `true`.\n- `pending_output_len_for_tests_reports_each_queued_input` gives the\nhot-join accessor a\npositive-count control instead of relying on a `<=` bound that passes\nfor any constant.\n\n## Verification\n\n```text\ncargo fmt --all -- --check                                       clean\ncargo clippy --workspace --all-targets --features tokio,json -- -D warnings        clean\ncargo clippy --workspace --all-targets --features tokio,json,hot-join -- -D warnings  clean\ncargo tfull                                                      3223 passed, 75 skipped\ncargo nextest run --workspace --features tokio,json,hot-join     3513 passed, 76 skipped\npython3 scripts/ci/agent-preflight.py --full --auto-fix          all checks passed\npython3 scripts/hooks/check-changelog-unreleased.py              pass\npython3 scripts/hooks/check-plan-hygiene.py                      pass\n```\n\nNo public API change; no wire-format change. `PLAN.md` remains empty.",
          "timestamp": "2026-11-16T04:14:49Z",
          "tree_id": "60193421d9c9961aee3e9caeadd512473432405c",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/41e7476e3db03f54d89237de00a610fa2a5697a4"
        },
        "date": 1794803191626,
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 19,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 30,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 99,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 19,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 26,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 36,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 106,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 19,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 20,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 58,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 78,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 103,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 97,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 139,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 212,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 173,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 255,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 406,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 102,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 123,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 152,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 150,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 192,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 273,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 301,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 379,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 531,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 241,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 317,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 461,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 575,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 73,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 94,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 463,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 788,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1194,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 37,
            "range": "± 9",
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
            "value": 57,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 202,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 692,
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
          "distinct": false,
          "id": "97e16a73ed70abcb54d0909a8e7eaa68b75e4816",
          "message": "test(spectator): pin catch-up boundary cases for mutation coverage (#437)\n\n## Summary\n\nTest-only follow-up to #436 (merged as `41e7476e`). Main's post-merge\ndiff-scope mutation run left one\nsurvivor after #436:\n\n```text\nMISSED src/sessions/p2p_spectator_session.rs:803:53: replace > with >= in SpectatorSession::advance_frame\n```\n\nThe catch-up decision is strict (\"more than `max_frames_behind`\"), so a\nlarger `catchup_speed` must\nnot apply when the spectator is exactly at the threshold.\n`spectator_at_the_catchup_threshold_advances_at_normal_speed`\npins exactly that boundary (red under `>=`: advances `Frame(1)` instead\nof `Frame(0)`).\n\nAlso pins the host-present compatibility half of the new terminal-drain\nbehavior, which previously\nonly had host-drained coverage: with a live host and `catchup_speed ==\n0`, `advance_frame` still\nreturns `Ok(empty)` without advancing. This makes the `hosts.is_empty()`\nbranch mutation-visible\n(red under an always-drain variant: `PredictionThreshold`).\n\nNo production code changes.\n\n## Verification\n\n```text\ncargo fmt --all -- --check                                       clean\ncargo nextest run -E 'test(spectator_at_the_catchup_threshold)|test(spectator_zero_catchup_with_a_live_host)|test(spectator_all_hosts_dropped_with_catchup_disabled)'  3 passed\ncargo tfull                                                      3223 passed, 75 skipped\ncargo nextest run --workspace --features tokio,json,hot-join     3513 passed, 76 skipped\npython3 scripts/ci/agent-preflight.py --full --auto-fix          all checks passed\n```",
          "timestamp": "2026-11-16T04:46:18Z",
          "tree_id": "40c9357f3b3649f7dee501eaaceddcddc107c3ba",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/97e16a73ed70abcb54d0909a8e7eaa68b75e4816"
        },
        "date": 1794805043886,
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
            "value": 24,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
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
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 125,
            "range": "± 2",
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
            "value": 78,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 96,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 126,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 144,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 199,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 289,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 252,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 357,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 530,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 142,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 156,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 186,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 257,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 309,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 401,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 480,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 577,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 766,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 321,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 440,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 616,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 801,
            "range": "± 8",
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
            "value": 133,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 395,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 662,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 939,
            "range": "± 48",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 36,
            "range": "± 2",
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 61,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 221,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 895,
            "range": "± 36",
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
        "date": 1794811674789,
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
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 17,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 22,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 76,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 18,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 17,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 98,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 15,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 20,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 17,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 44,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 54,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 59,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 77,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 114,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 114,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 167,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 229,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 56,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 83,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 75,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 113,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 134,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 174,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 228,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 266,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 324,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 192,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 217,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 302,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 397,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 57,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 78,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 365,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 592,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 883,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 25,
            "range": "± 7",
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
            "value": 11,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 115,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 424,
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
          "distinct": false,
          "id": "40e50dd0d4b9dcee2547b7944cc2031a7de9c815",
          "message": "test(ci): execute the determinism suite on aarch64 and 32-bit x86 (#440)\n\n## Summary\n\nAdds a `cross-arch-runtime` CI matrix that **executes** the\ndeterministic suite on the two\nnon-x86_64 architectures the determinism contract depends on:\n\n- **aarch64** under `qemu-user` (the existing `arm64-cross` job only\ncompiles).\n- **i686** natively, exercising the same 32-bit `usize` as `wasm32`.\n\nThe new lanes found and this PR fixes real test-suite defects:\n\n1. **The 32-bit suite did not compile.** `src/rng.rs`'s\n`test_gen_range_usize_large` forms\n`(u32::MAX as usize) + 1000`, a compile-time overflow on 32-bit. Gated\nto 64-bit targets;\n   the >`u32::MAX` range it tests cannot exist on 32-bit.\n2. **Four allocation-diagnostic tests asserted 64-bit `usize::MAX`.**\n`input_queue`,\n`input_bytes::zeroed`, `saved_states`, and `time_sync` expected\n`requested_elements:\nu64::MAX`; a 32-bit target widens `u32::MAX` instead. They now compare\nagainst\n   `u64::try_from(usize::MAX)`.\n3. **The replay byte-bound test was width-specific.** `1_000_000_000 *\nMIN_FRAME_ENCODED_LEN`\noverflows `usize` on 32-bit (\"overflows the byte bound\") but only\nexceeds the remaining\nbytes on 64-bit (\"exceeds ...\"). The test accepts either rejection form\nwhile still\n   requiring the `frames` field to be named.\n4. **The F17 reorder determinism guard was not deterministic off\nx86_64.** The 10-run guard\nfailed under aarch64 QEMU because the choreography used the default\nwall-clock protocol\ntimer: retry/pending-output timing changed which packets were sent\nduring the hold/release\nwindow on a slower host. It now injects `TestClock` (advanced 16 ms per\npoll) and fixed\n   protocol seeds, per the repository's protocol-test rule.\n\n`docs/specs/determinism-model.md` (and the wiki mirror) now records\naarch64 Linux and i686\nLinux as ✅ Verified with the exact CI scope. A CI contract test pins\nboth lanes, their\ntargets, runner/linker, and filters.\n\n## Evidence\n\n- **i686 native**, full workspace minus simulation/multi-process:\n  `2812 tests run: 2812 passed, 490 skipped` (exit 0).\n- **aarch64 QEMU**, workspace minus simulation/multi-process/compression\nproperty tests:\n  `2714 tests run: 2714 passed (4 slow), 590 skipped` (exit 0, 236 s).\n- aarch64 QEMU F17 regression + determinism guard: 2 passed (guard = 10\nruns in 7.6 s).\n- x86_64 targeted F17 tests and the five allocation/byte-bound tests:\npass.\n- `cargo tfull`: 3,229 passed / 75 skipped.\n- `cargo fmt`, `cargo clippy --workspace --all-targets --features\ntokio,json`, `actionlint`,\n122 CI contract tests, `agent-preflight --full`, plan/link/doc-claim\nchecks: clean.\n- `cargo mutants` on `hash` + `time_sync`: 52 mutants, 32 caught, 18\nunviable, 2 timeouts,\n  **0 missed**.\n- `fuzz_replay_decode`: 2,155,102 runs / 91 s, no crash.\n\n## Notes\n\n- The QEMU lane excludes proptest-heavy compression property tests (they\nexceed the native\nper-test budget under emulation; they still run on the native matrix)\nand the nightly\nsimulation/multi-process tiers, matching the `cargo tf` scope. The\ndedicated\n`ci-cross-arch` nextest profile keeps zero retries and allows a 240 s\nceiling for genuinely\n  slow QEMU tests.\n- No production behavior changes; this hardens the cross-platform\nverification surface that\n  guards the WASM/ARM pointer-width fixes from #439.\n\n## CI validation\n\nA `ci-rust` workflow dispatch on the PR head (`5a3ca32`, run\n\n[43823381040](https://github.com/Ambiguous-Interactive/fortress-rollback/actions/runs/43823381040))\ncompleted both new jobs successfully (2,812 i686 tests and 2,808\naarch64/QEMU tests, all passed):\n[Cross-Arch Runtime\n(i686-32bit)](https://github.com/Ambiguous-Interactive/fortress-rollback/actions/runs/43823381040/job/133151399692)\nand\n[Cross-Arch Runtime\n(aarch64-qemu)](https://github.com/Ambiguous-Interactive/fortress-rollback/actions/runs/43823381040/job/133151400218).",
          "timestamp": "2026-11-16T19:16:05Z",
          "tree_id": "008a40c308ec2e26933803aecbec524921165425",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/40e50dd0d4b9dcee2547b7944cc2031a7de9c815"
        },
        "date": 1794857351825,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 35,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 28,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 28,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 123,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 29,
            "range": "± 3",
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
            "value": 78,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 96,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 126,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 142,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 197,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 282,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 250,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 357,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 525,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 143,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 154,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 186,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 273,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 327,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 416,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 476,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 574,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 759,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 325,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 450,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 634,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 800,
            "range": "± 6",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 403,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 667,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 944,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 33,
            "range": "± 2",
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 61,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 220,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 905,
            "range": "± 10",
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
        "date": 1794869775789,
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 104,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 45,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 149,
            "range": "± 1",
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
            "value": 22,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 23,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 64,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 111,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 105,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 147,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 225,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 188,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 274,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 446,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 93,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 110,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 141,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 164,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 205,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 282,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 328,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 410,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 562,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 277,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 349,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 529,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 692,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 80,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 115,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 517,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 867,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1273,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 23,
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
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 56,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 202,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 782,
            "range": "± 1",
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
        "date": 1794884909263,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 85,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 102,
            "range": "± 10",
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 55,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 74,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 101,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 88,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 131,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 199,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 152,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 227,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 406,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 99,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 131,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 157,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 197,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 268,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 267,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 342,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 521,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 247,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 313,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 456,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 591,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 84,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 111,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 370,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 592,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 835,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 31,
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
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 52,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 207,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 809,
            "range": "± 3",
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
        "date": 1794893228807,
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
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 76,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 17,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 15,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 95,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 17,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 36,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 43,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 54,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 58,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 77,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 113,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 113,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 162,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 221,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 76,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 74,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 126,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 150,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 169,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 205,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 246,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 308,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 170,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 217,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 303,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 385,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 57,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 78,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 380,
            "range": "± 60",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 624,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 950,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 15,
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
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 32,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 121,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 549,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 28966,
            "range": "± 671",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 137320,
            "range": "± 2076",
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
        "date": 1794896140579,
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 105,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 45,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 149,
            "range": "± 1",
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 22,
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
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 62,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 81,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 114,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 102,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 143,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 230,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 206,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 292,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 462,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 92,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 113,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 140,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 187,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 232,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 306,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 323,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 404,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 560,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 244,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 341,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 519,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 678,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 80,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 116,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 540,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 932,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1365,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 23,
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
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 65,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 237,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 930,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 46123,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 221233,
            "range": "± 374",
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
        "date": 1794900215814,
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
            "value": 23,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 85,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 109,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 22,
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 55,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 74,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 100,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 87,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 129,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 204,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 152,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 233,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 376,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 84,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 114,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 140,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 155,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 184,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 275,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 267,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 347,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 498,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 246,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 330,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 479,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 604,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 111,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 359,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 562,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 796,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 32,
            "range": "± 2",
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
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 55,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 210,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 875,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 38050,
            "range": "± 199",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 189690,
            "range": "± 1615",
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
        "date": 1794901911855,
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
            "value": 24,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 85,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 113,
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
            "value": 22,
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
            "value": 54,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 97,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 87,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 128,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 192,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 152,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 231,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 388,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 85,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 100,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 125,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 146,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 182,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 262,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 267,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 338,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 494,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 247,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 327,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 477,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 587,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 83,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 110,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 371,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 585,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 816,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 32,
            "range": "± 2",
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
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 55,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 210,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 884,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 37432,
            "range": "± 189",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 186964,
            "range": "± 909",
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
        "date": 1794908142490,
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
            "value": 24,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 28,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 47,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 124,
            "range": "± 4",
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
            "value": 78,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 97,
            "range": "± 2",
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
            "value": 139,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 202,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 293,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 253,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 356,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 549,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 143,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 157,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 191,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 276,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 311,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 430,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 481,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 583,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 789,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 321,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 442,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 626,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 783,
            "range": "± 12",
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
            "value": 136,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 459,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 714,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1042,
            "range": "± 50",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 36,
            "range": "± 2",
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 62,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 222,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 986,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 49068,
            "range": "± 686",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 232428,
            "range": "± 1108",
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
        "date": 1794910592013,
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
            "value": 24,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
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
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 124,
            "range": "± 2",
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
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 78,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 97,
            "range": "± 1",
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
            "value": 144,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 199,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 282,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 253,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 356,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 531,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 143,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 156,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 187,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 258,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 310,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 397,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 480,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 579,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 766,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 327,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 452,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 606,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 797,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 100,
            "range": "± 2",
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
            "value": 432,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 708,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 974,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 35,
            "range": "± 2",
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 74,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 260,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1174,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 49529,
            "range": "± 179",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 244641,
            "range": "± 4323",
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
        "date": 1794956760106,
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
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 17,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 93,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 70,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 56,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 71,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 128,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 107,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 153,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 210,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 54,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 60,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 70,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 123,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 140,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 180,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 195,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 235,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 285,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 214,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 202,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 302,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 365,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 54,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 360,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 595,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 908,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 23,
            "range": "± 4",
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
            "value": 10,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 114,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 426,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 26891,
            "range": "± 77",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 127623,
            "range": "± 258",
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
        "date": 1794958250203,
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 118,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 43,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 127,
            "range": "± 1",
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 74,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 98,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 128,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 124,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 172,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 266,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 222,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 318,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 491,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 104,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 127,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 161,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 181,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 235,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 318,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 338,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 428,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 597,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 285,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 372,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 533,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 680,
            "range": "± 7",
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
            "value": 589,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 996,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1514,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 27,
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
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 67,
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
            "value": 951,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 45831,
            "range": "± 195",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 218133,
            "range": "± 909",
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
        "date": 1794963232305,
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
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 44,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 110,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 50,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 130,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 29,
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
            "value": 71,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 97,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 130,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 112,
            "range": "± 0",
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
            "value": 264,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 197,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 304,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 504,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 108,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 130,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 185,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 187,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 240,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 342,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 344,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 441,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 661,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 313,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 426,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 607,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 772,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 104,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 140,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 458,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 732,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1040,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 37,
            "range": "± 3",
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
            "value": 21,
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
            "value": 992,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 49781,
            "range": "± 198",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 243822,
            "range": "± 853",
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
        "date": 1794966435837,
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 99,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 36,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 106,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 21,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 64,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 85,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 111,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 104,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 152,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 238,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 185,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 283,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 433,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 104,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 124,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 140,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 153,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 199,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 266,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 281,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 359,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 515,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 235,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 310,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 456,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 574,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 73,
            "range": "± 1",
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
            "value": 494,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 841,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1275,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 23,
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
            "value": 60,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 210,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 806,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 39099,
            "range": "± 1795",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 185951,
            "range": "± 1542",
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
        "date": 1794969827413,
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
            "value": 24,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
            "range": "± 4",
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
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 124,
            "range": "± 5",
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
            "range": "± 1",
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
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 78,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 97,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 126,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 140,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 202,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 293,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 252,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 359,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 542,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 140,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 155,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 190,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 276,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 332,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 427,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 480,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 579,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 785,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 324,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 451,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 653,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 791,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 95,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 134,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 428,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 708,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 993,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 37,
            "range": "± 9",
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 65,
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
            "value": 962,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 50899,
            "range": "± 529",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 240478,
            "range": "± 1660",
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
        "date": 1794986054488,
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
            "value": 24,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 85,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 112,
            "range": "± 5",
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 55,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 97,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 87,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 127,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 205,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 153,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 232,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 374,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 98,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 130,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 145,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 182,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 259,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 267,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 343,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 499,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 239,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 330,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 463,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 586,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 83,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 111,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 356,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 569,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 804,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 30,
            "range": "± 2",
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
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 53,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 199,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 765,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 37657,
            "range": "± 350",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 184886,
            "range": "± 855",
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
        "date": 1794993815915,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 24,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 124,
            "range": "± 7",
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
            "value": 78,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 99,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 132,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 140,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 201,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 280,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 252,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 361,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 548,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 161,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 156,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 193,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 276,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 332,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 421,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 479,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 581,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 771,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 343,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 443,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 637,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 820,
            "range": "± 9",
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
            "value": 133,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 432,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 694,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1012,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 37,
            "range": "± 2",
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 67,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 235,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 957,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 49774,
            "range": "± 1259",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 235092,
            "range": "± 1567",
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
        "date": 1794997032135,
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
            "value": 24,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 86,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 39,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 112,
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
            "value": 22,
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
            "value": 54,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 73,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 95,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 87,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 131,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 203,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 153,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 229,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 375,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 84,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 99,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 127,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 145,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 183,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 291,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 267,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 339,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 496,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 247,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 319,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 467,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 598,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 91,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 118,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 363,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 572,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 831,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 31,
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
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 192,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 810,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 37597,
            "range": "± 1168",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 188769,
            "range": "± 502",
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
        "date": 1794998863142,
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
            "value": 24,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 47,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 121,
            "range": "± 2",
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 78,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 97,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 126,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 144,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 200,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 279,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 251,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 357,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 527,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 141,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 155,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 205,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 257,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 309,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 405,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 481,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 581,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 773,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 319,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 436,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 620,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 795,
            "range": "± 6",
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
            "value": 133,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 428,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 708,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 985,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 36,
            "range": "± 2",
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 64,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 232,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 919,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 49020,
            "range": "± 133",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 234298,
            "range": "± 1713",
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
        "date": 1795002240393,
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
            "value": 24,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
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
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 32,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 125,
            "range": "± 4",
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
            "value": 24,
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
            "value": 78,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 97,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 131,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 145,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 204,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 292,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 271,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 380,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 567,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 140,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 155,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 188,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 275,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 330,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 426,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 479,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 579,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 785,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 413,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 439,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 650,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 809,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 96,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 133,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 443,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 723,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1008,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 34,
            "range": "± 2",
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 64,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 232,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 922,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 49050,
            "range": "± 162",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 234159,
            "range": "± 2268",
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
        "date": 1795009852271,
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 105,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 45,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 150,
            "range": "± 1",
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 22,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 24,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 63,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 81,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 110,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 120,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 162,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 222,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 185,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 270,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 443,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 94,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 126,
            "range": "± 55",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 153,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 164,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 207,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 285,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 328,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 412,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 558,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 242,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 339,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 515,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 663,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 80,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 117,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 563,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 939,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1379,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 23,
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 60,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 216,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 854,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 45939,
            "range": "± 356",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 226615,
            "range": "± 25177",
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
        "date": 1795013037151,
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 99,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 35,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 113,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 62,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 83,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 111,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 102,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 148,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 225,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 199,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 287,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 433,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 104,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 110,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 139,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 154,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 201,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 293,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 282,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 359,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 510,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 232,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 314,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 457,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 558,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 73,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 96,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 498,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 853,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1292,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 23,
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
            "value": 59,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 209,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 805,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 38645,
            "range": "± 340",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 182133,
            "range": "± 698",
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
        "date": 1795034334781,
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
            "value": 24,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 86,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 115,
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
            "value": 22,
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 54,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 73,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 104,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 86,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 134,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 211,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 154,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 235,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 377,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 85,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 113,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 144,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 152,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 191,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 268,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 290,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 360,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 506,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 254,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 373,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 495,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 625,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 84,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 110,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 350,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 563,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 802,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 32,
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
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 192,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 750,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 37737,
            "range": "± 362",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 187036,
            "range": "± 1009",
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
        "date": 1795074967381,
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
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 72,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 17,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 93,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 52,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 56,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 72,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 112,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 109,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 161,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 220,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 54,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 61,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 71,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 103,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 127,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 160,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 197,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 241,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 292,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 158,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 214,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 284,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 366,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 54,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 72,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 360,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 600,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 903,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 15,
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
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 109,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 398,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 27679,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 129316,
            "range": "± 459",
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
          "id": "52052fe82ea5fd1a3c4e08bfec87f36e2b3a0d51",
          "message": "fix: keep periodic hot-join cycles live through stale freeze gossip (#489)\n\n## Description\n\nFixes the red `CI - Network Tests (Nightly)` soak on `main`. The\n4,000,000-frame\n\n`network::soak::four_million_frame_soak_preserves_bounds_replay_and_lifecycle`\nstopped at the\n36th periodic hot-join (confirmed frame 3,600,006): slot 0 stayed in\n`HotJoining` for millions of\npolls while survivors 1-3 remained `Running`, so the mesh never reached\nthe target.\n\n### Root causes\n\nDiagnostics (connect-status dumps + observer violations at the stall)\nshowed both survivors\nrejecting the coordinator's `ReactivateSlot` because a running peer\nstill claimed slot 0\nconnected, while the coordinator repeatedly aborted the serve\n(`last_saved_frame` moved off the\npinned snapshot during rollback repair) and the joiner retried forever.\n\n1. **A committed drop closed only a REOPENED `pending_reactivation`.** A\nnot-yet-reopened pending\nsurvived the drop. Its shield exempts the slot from the\ndisconnect-convergence fold and\nreopened attempts are never superseded, so the slot stayed connected and\nevery later directive\nwas rejected. `close_reopened_reactivation_for_drop_targets` now closes\n**any** pending for a\ndrop target: aborted (restore the pre-reopen freeze) for a\nnever-reopened attempt, committed\n   close preserved for a reopened one (the D17 implied-commit behavior).\n2. **Stale connected gossip above the local freeze blocked\nconvergence.** A prediction-capped\nsurvivor stops sending inputs with its pre-drop connected claim, and\npeers kept waiting for it\nto converge. A connected claim whose frame is **above** the survivor's\nown freeze cannot\nundercut that freeze, and the claiming peer's own gate still requires it\nto be locally frozen\nbefore it can reopen. The gate now blocks only on connected claims\nat/below the local freeze\n(or NULL claims), and also skips the joining endpoint's own self-claim.\n\n### Tests\n\n- `drop_commit_closes_a_not_yet_reopened_reactivation` — red without fix\n1.\n- `npeer_survivor_ignores_a_stale_connected_claim_above_its_freeze` —\nred without fix 2.\n-\n`npeer_survivor_ignores_the_joiners_own_connected_claim_for_convergence`\n— red without the\n  joiner self-claim skip.\n- `d17_new_drop_fence_outlives_delayed_prior_join_commit` still passes\n(reopened close semantics\n  preserved).\n\n### Verification\n\n- `FORTRESS_SOAK_TARGET_FRAMES=...` 3.7M release soak: passed in 330s\n(previously wedged at 3.6M).\n- Full `cargo nextest run --profile ci-network-nightly --release\n--run-ignored ignored-only -E\n\n'test(network::soak::four_million_frame_soak_preserves_bounds_replay_and_lifecycle)'\n--test\n  network --features hot-join`: **1 passed** in 336s.\n- `cargo nextest run --lib` 1947 passed; `--features hot-join` 2183\npassed.\n- `cargo nextest run --test simulation` 472 passed; `cargo tfull` 3326\npassed.\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings` clean;\n  `python3 scripts/ci/agent-preflight.py` passed.\n\n## Type of Change\n\n- [x] Bug fix (non-breaking change that fixes an issue)\n- [x] Test (adding or updating tests)\n\n## Related Issues\n\nRelates to #486 (merged nightly regression source), #488.",
          "timestamp": "2026-11-19T08:33:13Z",
          "tree_id": "7c6475be7b880b79fcd1ea90a4b0f888bb1cfd96",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/52052fe82ea5fd1a3c4e08bfec87f36e2b3a0d51"
        },
        "date": 1795077940185,
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
            "value": 24,
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
            "value": 24,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 106,
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
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 48,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 123,
            "range": "± 2",
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
            "value": 78,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 98,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 132,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 139,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 201,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 292,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 252,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 359,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 548,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 161,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 175,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 210,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 257,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 314,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 411,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 481,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 581,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 786,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 324,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 450,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 630,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 792,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 97,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 133,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 433,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 697,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1005,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 36,
            "range": "± 2",
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 66,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 235,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 928,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 49397,
            "range": "± 173",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 232365,
            "range": "± 1541",
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
          "id": "73100949072af6bfd9c58c2204753d181b7d9d3c",
          "message": "test: close coordinated-drop mutation gaps from #486 (#490)\n\n## Description\n\nFollow-up to #486/#489. The post-merge diff-scoped mutation gate on\n#486's tree reported 10 missed mutants in the coordinated-drop donor and\nunseen-commit paths. The diff gate requires **zero** misses, so this PR\nadds the missing tests and two small guard simplifications.\n\n### Production changes\n\n- The unseen-evidence source set no longer has a redundant `len <\nnum_players` guard. The journal is already bounded by `num_players` and\nthe set holds transport-attributed handles.\n- The stale-commit check now distinguishes known from unknown operations\nwith an explicit branch. A delayed duplicate stays rejected even when\nrepeated by a second endpoint, while known operations keep their normal\nordering rules.\n\n### Tests added\n\n- `announced_donor_must_cover_the_required_start`: a lower-handle\nannouncement that does not cover the required start cannot displace the\ncovering donor.\n- `a_locked_grid_rejects_a_second_announced_donor`: each of the four\nlocked-grid components (`backfill_donor`, `expected_backfill_chunks`,\n`backfill_chunks`, `backfill_grid_source`) rejects another announced\ndonor's chunk.\n- `announced_donor_wrong_chunk_count_is_discarded`: a mismatched chunk\ncount is discarded at the announced-grid shape check, not failed closed\nlater.\n-\n`locked_grid_rejects_an_announced_fallback_for_a_non_selected_inventory_donor`:\nthe inventory branch's no-grid-locked fallback cannot override a locked\ngrid.\n- `committed_fence_mismatches_are_rejected_or_ignored_correctly`:\nsame-operation/different-cut is a conflict; different-operation/same-cut\nis not an exact duplicate.\n- `closed_operation_commit_is_ignored_without_halting`: a corroborated\ncommit for an operation already in the closed journal stays a no-op.\n- `partial_commit_mismatch_fails_closed_and_stages_a_matching_commit`: a\ndiffering cut alone conflicts, and a matching commit is staged on a\nnon-committed attempt.\n- Assertions pin the replay watermark's single-entry/max-cut behavior,\nthe corroborated-evidence cleanup, and the total installed-backfill\ncounter.\n\n### Verification\n\n- `cargo nextest run --workspace --features\ntokio,json,hot-join,violation-census --no-fail-fast` -> **3638 passed, 0\nfailed**, 77 skipped.\n- `cargo clippy --workspace --all-targets --features\ntokio,json,hot-join,violation-census -- -D warnings` -> clean.\n- Targeted `cargo mutants --file src/sessions/p2p_session.rs --re\n'coordinated_drop_accept_backfill|unseen_commit_corroborated|record_closed_cut|coordinated_drop_accept_commit'\n--in-place --timeout 300 -- --lib` -> **79 mutants: 70 caught, 9\nunviable, 0 missed** (29m).\n- The earlier full diff run selected 171 mutants; the tests here cover\nall 10 reported misses (the four locked-grid `&&` sites, announced\nshape/count, inventory fallback, unseen source guard, journal retain,\nwatermark update, stale predicate, and the diagnostic counter).\n\n## Type of Change\n\n- [x] Test (adding or updating tests)\n- [x] Bug fix (non-breaking guard simplification)\n\n## Checklist\n\n- [x] Zero-panic policy preserved\n- [x] `cargo fmt && cargo clippy ... -D warnings` clean\n- [x] `cargo nextest run` passes\n- [x] No public API change; diagnostics stay `#[doc(hidden)]`\n\n## Related Issues\n\nFollow-up to #486, #489. Relates to #488.",
          "timestamp": "2026-11-19T10:20:53Z",
          "tree_id": "8264ed561c54dd3577d74da6a981332ddb6363bf",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/73100949072af6bfd9c58c2204753d181b7d9d3c"
        },
        "date": 1795084444640,
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
            "value": 24,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
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
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 125,
            "range": "± 4",
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
            "value": 78,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 97,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 131,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 143,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 205,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 296,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 257,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 359,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 572,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 143,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 156,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 187,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 279,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 336,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 435,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 483,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 583,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 791,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 324,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 440,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 629,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 807,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 97,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 133,
            "range": "± 2",
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
            "value": 732,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1020,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 36,
            "range": "± 2",
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 66,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 237,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 923,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 50457,
            "range": "± 97",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 234171,
            "range": "± 1459",
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
          "id": "f756decdd54f268fc5b252aac628257123fe3eaa",
          "message": "test: pin announced-donor installation and recover closed drop commits (#491)\n\n## Summary\n\nCloses #488 and resolves the still-valid review findings from the\nrecently merged coordinated-drop PRs.\n\n- **#488 — announced-donor installation end-to-end.** Adds generated\nlossy lifecycle row `(seed 43, n=3, Mild)` to\n`lossy_graceful_removal_rows_keep_one_history_and_recover`, asserts that\nan announced chunk actually installs (not merely that the announcement\nis accepted), pins the inventory-donor chunk path too, and verifies a\nserialize/deserialize replay produces the same trace hash and\nannounced-install counters.\n- **#489 follow-up — disconnected-target certificates.**\n`coordinated_drop_apply_commit` now accepts a target that is already\ndisconnected at the exact committed generation, closes a\nnot-yet-reopened reactivation without restoring/re-freezing the queue\nthe drop is about to freeze, and installs the membership fence.\nRegression drives the full commit path.\n- **#458 follow-up — commit recovery after quorum close.** Recently\napplied certificates are retained in a bounded per-player journal. A\nparticipant that missed the commit burst and retries `Ready` after every\nsurvivor closed the operation now receives the certificate again instead\nof timing out. A mismatched digest never replays it.\n- **Test adaptation.** The commit responder changes coordinated-drop\ncompletion timing, so the PCT lagging-survivor regression now uses a\nseed that preserves the required interleaving and re-pins its exact\nconfirmed vector.\n\n`PLAN.md` is empty after closing #488. No public API change; changelog\nentries describe the two pre-existing liveness fixes.\n\n## Validation\n\n- `cargo nextest run` — 3286 passed, 76 skipped\n- `cargo nextest run --features hot-join` — 3587 passed, 77 skipped\n- `cargo clippy --workspace --all-targets --features\ntokio,json,hot-join`\n- `cargo fmt --check`\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix`",
          "timestamp": "2026-11-19T17:39:15Z",
          "tree_id": "725a2831f2773f1c5eecf9bd076497a0b751efeb",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/f756decdd54f268fc5b252aac628257123fe3eaa"
        },
        "date": 1795110728408,
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 105,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 149,
            "range": "± 1",
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 62,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 109,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 102,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 145,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 221,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 181,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 271,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 440,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 97,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 112,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 139,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 164,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 208,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 281,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 322,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 407,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 557,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 249,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 337,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 505,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 658,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 80,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 115,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 559,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 933,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1374,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 22,
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
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 57,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 201,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 769,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 45832,
            "range": "± 163",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 221210,
            "range": "± 999",
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
          "id": "66359201746b10e2f11cfc8fc543d674698297e7",
          "message": "fix: keep disconnected-target freeze at the certified cut (#492)\n\n## Summary\n\nFollow-up to merged #491. Its post-merge mutation-testing run found that\nthe disconnected-target freeze could still be mined down below the\ncertified cut by the ordinary disconnect fold, and the diff-scope\nmutants for the pre-fold workaround were not all covered.\n\n- Remove the pre-fold status override.\n- After `disconnect_player_at_frames`, re-assert `commit.cut` as every\ntarget's frozen status frame and frozen input value. This makes the\nfinal status/frozen value authoritative even when the target already\ncarried a lower disconnect high-water.\n- Keep the certificate journal duplicate test, which rejects\nre-recording the same commit.\n- Update the existing unreleased changelog entry to state the certified\nfrozen value guarantee.\n\nNo API change.\n\n## Validation\n\n- `cargo nextest run --features hot-join` targeted regressions pass.\n- `cargo nextest run` full default suite and `cargo nextest run\n--features hot-join` passed on #491; this follow-up keeps the same tests\nplus the corrected freeze assertions.\n- `cargo fmt --check`, changelog hook, and `agent-preflight.py\n--auto-fix` pass.\n- `cargo mutants --in-diff` reports no viable surviving mutants for this\nfollow-up diff.",
          "timestamp": "2026-11-19T18:20:24Z",
          "tree_id": "c68e7bf6e1f37163a882824cceed83062ff22382",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/66359201746b10e2f11cfc8fc543d674698297e7"
        },
        "date": 1795113237573,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 24,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
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
            "value": 30,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 48,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 125,
            "range": "± 2",
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
            "value": 24,
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
            "value": 78,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 98,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 127,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 144,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 204,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 281,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 251,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 355,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 571,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 143,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 156,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 190,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 276,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 338,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 430,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 479,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 582,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 766,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 320,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 438,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 605,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 794,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 94,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 131,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 441,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 732,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 996,
            "range": "± 32",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 34,
            "range": "± 2",
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
            "value": 235,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1011,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 49781,
            "range": "± 367",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 239665,
            "range": "± 3233",
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
          "id": "05998067b77cd45e1133cf5b354bb6e32208a22a",
          "message": "test: pin cross-arch hot-join traces and wasm removal paths (#494)\n\n## Summary\n\nCloses two cross-platform coverage gaps without changing production\nbehavior.\n\n### N-peer hot-join trace on non-x86_64 targets\n\nThe `cross-arch-runtime` matrix compiles with `--features tokio,json`,\nso the feature-gated `ScheduleEvent::HotJoin` path never executed under\nqemu-aarch64 or on 32-bit i686. The new\n`hot_join_trace_matches_checked_in_cross_architecture_digest` pins the\ncanonical `clean_npeer_hot_join_schedule(4)` trace\n(`0xe493_827c_d108_44b0`, confirmed `[859, 860, 860, 860]`) and a\ndedicated CI step runs it in both cross-arch lanes.\n\n### Explicit graceful removal in the wasm runtimes\n\nThe browser and Godot probes exercised synchronization, replay, hashing,\ncompression, and tab suspension, but neither executed a coordinated drop\ncertificate or the live-target commit retransmission path.\n\n- `explicit_graceful_removal_certifies_in_browser`\n(wasm32-unknown-unknown) synchronizes a two-peer `ContinueWithout` pair,\nadvances four frames, removes the remote player, requires `PeerDropped`,\nrequires the target to apply its committed certificate and stop, and\nrequires the survivor to keep advancing on the frozen slot.\n- `run_removal_probe` performs the same flow under the Emscripten\nGDExtension build and reports `removal_certified: true`; both threaded\nand non-threaded Playwright rows assert it.\n\n## Validation\n\n- Extra nightly fleet seed sweep, 1,000 fresh seeds across all 8 shards:\n`8 passed` (363 s).\n- Pinned hot-join trace: x86_64 pass, i686 native pass, qemu-aarch64\npass (5.4 s).\n- Browser runtime in headless Chrome 158 (wasm32-unknown-unknown): `8\npassed; 0 failed`, including the new removal test.\n- `cargo nextest run --workspace --features tokio,json`: 3,334 passed,\n76 skipped.\n- `cargo nextest run --workspace --features tokio,json,hot-join`: 3,637\npassed, 77 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json,hot-join\n-- -D warnings`: clean.\n- `cargo +nightly-2026-08-26 clippy --manifest-path\ntests/godot-emscripten/Cargo.toml --locked --all-targets --all-features\n-- -D warnings`: clean.\n- `cargo check -p wasm-browser-smoke --target wasm32-unknown-unknown\n--lib --tests`: pass.\n- `actionlint .github/workflows/ci-rust.yml`: pass.\n- `python3 scripts/ci/agent-preflight.py --auto-fix`: all checks passed.\n\n## Review audit\n\nRe-checked the review threads on every PR opened in the last five days\n(#414 through #492). The previously unresolved items from #474, #479,\n#481, #486, #489, and #491 are addressed in current `main`; #492 had\nzero review threads. No follow-up production change was required.\n\n## Follow-up\n\n#493 tracks the remaining wasm runtime gap: the N-peer hot-join\nhandshake/snapshot path is still native-only at runtime. It is listed in\n`PLAN.md` under Future with a concrete exit criterion.\n\nNo public API or user-observable behavior changes; no changelog entry.",
          "timestamp": "2026-11-19T19:52:21Z",
          "tree_id": "18549dd53d0b774973b8498df6743aca9ec6e0a8",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/05998067b77cd45e1133cf5b354bb6e32208a22a"
        },
        "date": 1795118646070,
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
            "value": 24,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 86,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 39,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 116,
            "range": "± 3",
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 22,
            "range": "± 1",
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
            "value": 54,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 97,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 87,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 126,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 193,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 152,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 229,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 374,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 102,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 125,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 146,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 184,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 253,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 266,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 338,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 494,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 227,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 337,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 469,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 618,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 109,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 357,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 573,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 807,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 47,
            "range": "± 11",
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
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 193,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 757,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 38214,
            "range": "± 1447",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 185712,
            "range": "± 875",
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
          "id": "d7abb296fd0af16f1ce7ff43298fa8cd50fc9bf9",
          "message": "test: drive the chaos latency floor from the injected clock (#495)\n\n## Summary\n\nFixes the post-merge main CI failure in `Miri (macOS - core)` on\n`0599806`:\n\n```text\nthread 'network::chaos_socket::tests::test_latency_minimum_delivery_time_data_driven' panicked:\nassertion `left == right` failed: [high_jitter] Packets delivered too early!\nmin_delivery=100ms, jitter=100ms, elapsed=129.54ms\n```\n\n## Root cause\n\nThe data-driven floor test queued the packets, took a wall-clock\n`Instant`, and asserted that a second receive immediately after the\nfirst delivered nothing. That assumption is only true while the test\nitself finishes inside the minimum delay. Miri's interpreter on macOS\ntook 129.54 ms between the two receives, so the 100 ms packet was\nalready due; the socket behaved correctly and the test reported a false\npositive.\n\n## Fix\n\nThe case now installs `TestClock` through `ChaosSocket::with_clock`,\nadvances it to exactly one millisecond below `latency - jitter`, and\nonly then asserts that no packet is ready. Test execution speed can no\nlonger affect the assertion on any target or instrumentation level. The\nmaximum-delivery sibling already used the same pattern.\n\n## Validation\n\n- `cargo nextest run --lib network::chaos_socket` -> 57 passed.\n- `cargo nextest run --workspace --features tokio,json` -> 3,334 passed,\n76 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings` -> clean.\n- `cargo fmt --all -- --check` -> clean.\n- Pre-commit hooks -> pass.\n\nNo production behavior changes; test-only.",
          "timestamp": "2026-11-19T20:28:23Z",
          "tree_id": "537f9d4bc2860c6bde7c55b449a5b8392c3f2f0b",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/d7abb296fd0af16f1ce7ff43298fa8cd50fc9bf9"
        },
        "date": 1795120823255,
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
            "value": 23,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 85,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 39,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 112,
            "range": "± 3",
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
            "value": 34,
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
            "value": 55,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 85,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 101,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 87,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 144,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 196,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 152,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 229,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 404,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 84,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 112,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 130,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 156,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 196,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 267,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 267,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 338,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 494,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 240,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 342,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 471,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 592,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 82,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 109,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 350,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 578,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 807,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 39,
            "range": "± 4",
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
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 193,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 755,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 38107,
            "range": "± 168",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 188177,
            "range": "± 2738",
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
          "id": "e442b7b0dacf8b4be15be67e459c3f00a682f25d",
          "message": "test: exercise hot-join membership in the wasm browser and Godot probes (#496)\n\n## Summary\n\nCloses #493. Adds the missing runtime coverage of the feature-gated\nhot-join membership path to both wasm probes. Hot join was the last\nmembership transition that only ran natively: snapshot capture/serve,\nsnapshot apply, activation-frame gating, and joined-slot input\ncontribution were never executed at runtime on either wasm target.\n\n### Browser (`wasm32-unknown-unknown`, real headless Chrome 158)\n\n- `hot_join_activates_and_confirms_joined_slot_in_browser`: a host runs\nsolo with a reserved slot for three frames; a joiner completes the\nsnapshot handshake through polling only (injected virtual clock, no\nsleeps); the joiner's first advance returns exactly one `LoadGameState`;\nboth peers then advance six frames. The final two confirmed frames must\nagree across peers and carry distinct non-default joined-slot inputs,\nproving the activation-frame gate and post-join input stream.\n- The smoke crate gains a non-default `hot-join` feature forwarding to\n`fortress-rollback/hot-join`; the `ci-rust.yml` browser runtime lane and\nthe `ci-pr.yml` wasm check opt in. Keeping it non-default prevents\nworkspace feature unification from silently switching the library into\nits hot-join configuration for unrelated builds.\n\n### Godot (`wasm32-unknown-emscripten`, threaded + nothreads)\n\n- `run_hot_join_probe` performs the same flow inside the 32-bit\nGDExtension build and reports `hot_join_activated`; both Playwright rows\nassert it. This exercises snapshot serialization, apply, activation, and\njoined-slot confirmation under the Emscripten runtime.\n\n### Feature-unification hardening\n\n- `network-test-peer` now always links `hot-join` and its exhaustive\n`FortressEvent` match no longer cfg-gates `JoinRequested`/`PeerJoined`.\nA workspace member enabling the library feature could previously leave\nthe peer's match non-exhaustive (surfaced by `cargo tfull --workspace`).\n\n## Validation\n\nBrowser runtime, actual Chrome for Testing 158 headless, full suite:\n\n```text\nrunning 9 tests\ntest tests::hot_join_activates_and_confirms_joined_slot_in_browser ... ok\ntest result: ok. 9 passed; 0 failed; 0 ignored; 0 filtered out\n```\n\nGodot Emscripten runtime (Emscripten 4.0.20, Godot 4.7.1, threaded and\nnon-threaded web exports, Playwright + xvfb):\n\n```text\nRunning 3 tests using 1 worker\n  ✓ console policy only admits the exact threaded dlink warning\n  ✓ threaded Godot export completes Fortress quality probes (4.3s)\n  ✓ nothreads Godot export completes Fortress quality probes (2.4s)\n  3 passed (7.5s)\n```\n\nFull suite and lint:\n\n```text\ncargo c: clean\ncargo tfull: 3637 tests run: 3637 passed (1 slow), 77 skipped\ncargo +nightly-2026-08-26 clippy --manifest-path tests/godot-emscripten/Cargo.toml\n  --locked --all-targets --all-features -- -D warnings: clean\ncargo check -p wasm-browser-smoke --features hot-join --target wasm32-unknown-unknown\n  --lib --tests: pass\nactionlint .github/workflows/ci-rust.yml .github/workflows/ci-pr.yml: clean\npython3 scripts/ci/agent-preflight.py --full --auto-fix: all checks passed\n```\n\nLatest `main` CI (`d7abb296`): 192 check runs, 185 success, 7 skipped, 0\nfailures.\n\n## Review audit\n\nAll PRs opened in the last five days (#414-#495) were re-checked. Every\nreview thread is resolved; the final OpenAI review finding count is zero\nafter follow-up commits (#479's coordinated-removal findings were closed\nby #480/#481, #474's by #475/#476). No further production follow-up was\nrequired.\n\n## Follow-up\n\n- `PLAN.md` now has an empty future queue; #493 is the last tracked\ncross-platform runtime gap.\n- Dependency majors (`bincode-next` 3.x, `parking_lot` 0.13,\n`serial_test` 4, `z3` 0.23) remain intentionally pinned to the frozen\nwire format / verification toolchain / MSRV and are out of scope for\nthis test-only change. No open dependency PRs exist.\n\nNo public API or user-observable behavior changes; no changelog entry.",
          "timestamp": "2026-11-19T22:03:17Z",
          "tree_id": "0959d5cebfa1fbd875c34705bcd21c37fa4a6550",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/e442b7b0dacf8b4be15be67e459c3f00a682f25d"
        },
        "date": 1795126598407,
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
            "value": 24,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 85,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 101,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 21,
            "range": "± 1",
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 55,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 75,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 101,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 87,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 128,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 183,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 152,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 244,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 390,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 84,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 101,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 140,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 156,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 198,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 260,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 275,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 343,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 516,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 239,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 320,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 464,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 595,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 82,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 109,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 349,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 573,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 807,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 49,
            "range": "± 2",
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
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 192,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 756,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 37458,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 187362,
            "range": "± 1713",
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
          "id": "74741dcfdf6bff55825afd9a668b322ef38aaf2d",
          "message": "test: pin isolated-coordinator CAP progress and correct closed residual docs (#498)\n\n## Description\n\nFollow-up to the coordinated-graceful-drop work from #458 through #496.\n\n- The `P2PSession::remote_slot_confirmed_bound` rustdoc still described\nthe N≥4 double-failure-relay residual as a deferred S41 fix and cited a\nconvergence test that no longer exists. The sequence-numbered\n`FloorRequest`/`FloorReply` round has closed the residual since S55, so\nthe docs now point at the current hold/release unit guards and the\npartitioned-certificate choreographies. The matching test/TLA-index\nnarratives are corrected too.\n- Adds the missing CAP matrix row\n`isolated_coordinator_removal_still_commits_on_reachable_majority`: N=5\n`3/1/1` partition, removal requested by a reachable survivor, the\ndeterministic coordinator isolated. With the default 2-second disconnect\ntimeout the coordinator endpoint times out inside the bounded attempt,\nthe majority reconciles membership, certifies both the requested target\nand the departed coordinator slot, keeps confirming, and does not\ndiverge.\n- The variant where a longer configured `disconnect_timeout` keeps the\nisolated coordinator \"Running\" past the two-second operation deadline\nstill fail-closes the reachable majority. That is a real liveness\nlimitation, now tracked in #497 with a deterministic reproduction and\ncandidate designs, and queued in `PLAN.md` with the TLA-modeled failover\nexit criterion.\n\nReview feedback on the last five days of PRs was triaged\nthread-by-thread against current `main`: all 48 unresolved threads were\nanswered with the current code/test evidence and resolved. No\nuncorrected production finding remained.\n\n## Type of Change\n\n- [x] 📚 Documentation (changes to documentation only)\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` (test/docs only; no\nuser-facing change)\n- [ ] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n-\n`simulation::partition_sweep::isolated_coordinator_removal_still_commits_on_reachable_majority`\n(new)\n- Existing guards re-run:\n`p2p_n4_double_failure_certificate_partition_fails_closed`,\n`p2p_n5_double_failure_certificate_partition_fails_closed`,\n`confirmed_frame_post_prune_unreplied_relay_holds_at_last_confirmed`,\n`remote_slot_confirmed_bound_consumes_relay_floor_round_after_prune`\n- Full `cargo nextest run --workspace --all-targets --features\ntokio,json,hot-join,violation-census`: 3709 passed, 77 skipped\n(pre-change baseline)\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix`: all checks\npassed\n\n**Manual testing performed:**\n\n- Deterministic probe of the 5-second `disconnect_timeout` variant\n(documented in #497): all four majority peers end `Synchronizing` with\n`report_count=3, ready_count=0` timeouts.\n\n## Related Issues\n\nRelates to #445, #473, #497.",
          "timestamp": "2026-11-20T00:01:06Z",
          "tree_id": "0f5d949b73d6a10a586bc65e25dd42cae36a63ff",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/74741dcfdf6bff55825afd9a668b322ef38aaf2d"
        },
        "date": 1795133604617,
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
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 16,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 17,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 22,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 78,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 15,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 96,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 35,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 42,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 54,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 59,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 75,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 113,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 113,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 166,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 232,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 57,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 65,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 74,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 111,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 131,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 173,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 208,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 256,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 301,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 157,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 203,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 307,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 366,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 58,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 78,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 386,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 627,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 952,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 14,
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
            "value": 11,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 113,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 419,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 28698,
            "range": "± 641",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 137093,
            "range": "± 548",
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
          "id": "444dee34b1cbc2c87841039627ee9e93793ed888",
          "message": "fix: let a reachable majority fail over an isolated drop coordinator (#499)\n\n## Description\n\nFixes #497. A coordinated graceful removal could fail closed a reachable\nstrict majority when the deterministic coordinator was partitioned away\nand the configured `disconnect_timeout` exceeded the two-second\noperation deadline. The failure reproduced as `final_confirmed=[74,...]`\nwith every majority peer `Synchronizing` and `phase=Inventory,\nreport_count=3, ready_count=0`.\n\nThis adds a bounded coordinator failover:\n\n- A non-coordinator prepare is retained for a one-second grace (half the\ntwo-second operation deadline). Any drop-control message from the real\ncoordinator cancels the pending candidate.\n- After the grace, the deterministic lowest remaining participant may\nadmit the retained prepare as acting coordinator and drive the unchanged\nquorum/cut/backfill/ready/commit path.\n- The real coordinator never yields an active attempt. A later\nconflicting cut still fails closed.\n- The acting role only excuses the replaced coordinator from the\nmissing-witness rules; quorum, participant-set validation, cut digests,\nand ready transcripts are unchanged.\n- Requester adoption is handled both from the fallback prepare and from\na cut/Ready that arrives first.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` (`**Pre-existing:**`\nFixed)\n- [ ] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `isolated_coordinator_removal_commits_under_long_disconnect_timeout`:\nN=5 `3/1/1`, 5s timeout, both coordinator and target isolated. Majority\nstays `Running`, confirms past 400, certifies the requested target, no\ndivergence, exact replay.\n- `fallback_coordinator_takes_over_a_different_requesters_prepare`:\nrequester is not the fallback; peer 1 adopts peer 2's prepare.\n- `fallback_prepare_requires_a_participant_source_and_waits_for_grace`,\n`coordinator_prepare_cancels_a_pending_fallback`,\n`requester_adopts_the_fallback_after_the_grace`,\n`acting_coordinator_excuses_only_the_replaced_coordinator`.\n- Updated the N=4/N=5 double-failure guards to distinguish the pre-heal\nheld frontier from legitimate post-heal certification.\n- Red/green: the 5s row fails with the failover grace set to 10s\n(`majority survivor 1 must stay Running`), and passes with the 1s grace.\n\n**TLA+:**\n\n- `specs/tla/CoordinatorFailover.tla` + `CoordinatorFailover.cfg`:\ncoordinator partitioned, fallback liveness (`EventuallyFailoverCommits`)\n+ safety (`SafetyInvariant`).\n- `CoordinatorFailoverCompeting.tla` + cfg: real coordinator may choose\na competing cut; safety proves at most one cut commits.\n- Both run in `scripts/verification/verify-tla.sh`.\n\n**Manual/CI evidence:**\n\n- Full `cargo nextest run --workspace --all-targets --features\ntokio,json,hot-join,violation-census`: 3716 passed, 77 skipped.\n- `cargo clippy ... -D warnings`: clean.\n- Both rustdoc lanes pass.\n- Cross-platform `cargo check` for `wasm32-unknown-unknown`,\n`wasm32-wasip1`, `aarch64-unknown-linux-gnu`, and\n`i686-unknown-linux-gnu` with no-default-features and each isolated\nfeature (`sync-send`, `paranoid`, `json`, `hot-join`, `tokio`) passes.\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix`: all checks\npassed.\n\n## Related Issues\n\nFixes #497.",
          "timestamp": "2026-11-20T01:48:13Z",
          "tree_id": "c770845b7311b6dbe6ea0bff58727094526ee84f",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/444dee34b1cbc2c87841039627ee9e93793ed888"
        },
        "date": 1795140008811,
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
            "value": 24,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 85,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 111,
            "range": "± 6",
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 55,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 74,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 101,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 87,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 126,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 206,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 166,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 234,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 405,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 114,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 144,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 157,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 203,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 279,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 267,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 344,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 498,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 250,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 326,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 474,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 593,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 113,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 349,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 591,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 815,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 31,
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
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 192,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 815,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 38475,
            "range": "± 125",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 190476,
            "range": "± 571",
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
          "id": "57494b78761444facd5a5f4b0aaeceef66a5d3be",
          "message": "test: close coordinator-failover mutation survivors (#503)\n\n## Description\n\nFollow-up to #499. The post-merge mutation job on `main` (`444dee34`)\nreported **30 missed mutants** in the new coordinator-failover state\nmachine. This adds targeted unit guards so each survivor is killed, and\nremoves one redundant condition plus one equivalent same-operation\nbranch so they no longer generate mutants.\n\nCovered classes:\n\n- bounded `coordinator_heard_operations` journal (oldest eviction at\n`num_players`);\n- retained fallback prepare ordering (lower operation wins, higher does\nnot replace, retransmissions do not reset the first-seen deadline);\n- fallback promotion boundaries (not before the grace; never when quorum\nequals the participant count, e.g. two survivors);\n- admission identity validation (`acting` must be the deterministic\nfallback and the source must match);\n- the deterministic coordinator never yields an active attempt;\n- non-fallback sends do not become acting;\n- unrelated coordinator reports do not clear a candidate or mark an\nactive attempt heard;\n- matching coordinator reports do mark the active attempt heard;\n- the full cut/Ready adoption matrices (grace, sender identity,\ncoordinator-heard, recorded-operation suppression,\ndeterministic-coordinator exclusion).\n\n## Type of Change\n\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🐛 Bug fix (small condition simplification only; no behavior\nchange)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` (test-only change)\n- [ ] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n- Full `cargo nextest run --workspace --all-targets --features\ntokio,json,hot-join,violation-census`: 3719 passed, 77 skipped.\n- `cargo clippy --workspace --all-targets --features\ntokio,json,hot-join,violation-census -- -D warnings`: clean.\n- Targeted new tests: 10/10 pass.\n- Local `cargo mutants` run on the affected functions confirms the\npreviously reported survivors are killed (see conversation evidence).\n- `python3 scripts/ci/agent-preflight.py --full --auto-fix`: expected to\npass; CI will confirm.\n\n## Related Issues\n\nFollow-up to #499 and #497.",
          "timestamp": "2026-11-20T04:35:25Z",
          "tree_id": "6a6e4cc6792312ddd1ab1d525f6165f9028f69c5",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/57494b78761444facd5a5f4b0aaeceef66a5d3be"
        },
        "date": 1795149990594,
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
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 17,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 94,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/8",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 14,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 41,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 52,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 55,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 71,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 108,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 106,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 158,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 208,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 53,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 61,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 70,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 103,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 121,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 176,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 194,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 234,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 298,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 264,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 190,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 321,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 374,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 54,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 72,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 371,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 592,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 897,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 13,
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
            "value": 10,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 30,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 102,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 407,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 27720,
            "range": "± 344",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 129758,
            "range": "± 773",
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
          "id": "c7b3f7b1e9bd3b0fa5a1be42a0bc46040b9b9201",
          "message": "fix: keep coordinated-drop failover live after coordinator silence (#508)\n\n## Summary\n\nCloses the coordinated-drop failover liveness gaps found by a CAP review\nof the newly landed\ncoordinator-failover path, plus the commit-before-cut availability gap\nin the same certificate flow.\n\n1. **The failover grace is now a silence window, not a \"never heard\"\nlatch.** A single early\ncoordinator message used to set `coordinator_heard = true` forever, so a\ncoordinator that\ncommitted/reported once and then disappeared forced a reachable strict\nmajority to fail closed\nat the two-second deadline. The journal now stores the most recent\ncoordinator timestamp per\noperation; fallback admission, promotion, cut adoption, and Ready\nadoption require a full\none-second silence window, and every coordinator message refreshes it.\nAn alive coordinator\nkeeps priority; a late conflicting cut still fails closed; quorum,\ntarget sets, digests,\n   backfill, and transcripts are unchanged.\n2. **A relayed prepare now also reaches the deterministic fallback\ncandidate.** Previously a\nnon-coordinator relay forwarded only to the coordinator. With the\ncoordinator isolated and the\nrequester-to-fallback link broken, the fallback first learned of the\noperation when the relay\ntook over at the grace, starting a fresh grace that expired with the\noperation deadline. The\n   first copy now starts the fallback's timer.\n3. **A commit that overtakes the cut/Ready round is adopted instead of\nrejected.** A valid\n`Committed` report (or direct `Commit`) received while the local attempt\nis still in `Inventory`\nused to return `ConflictingHistory` or stage a cut-less pending commit,\nfailing a reachable\nparticipant closed or timing it out. The certified cut is now adopted,\nthe attempt re-enters\nbackfill to verify local history, and the committed reporter counts as\nits ready transcript.\n4. **Coordinator-heard evidence lives on the live attempt/candidate**,\nnot only\nin the bounded cross-operation journal, so journal eviction can never be\nmistaken for coordinator silence. A regression evicts the journal entry\nand\n   asserts the fallback stays blocked while the coordinator is alive.\n5. **Steady-state drop polling skips empty mailboxes** before cloning\nendpoint handle sets.\n\nNo wire-format change. Out-of-scope findings are tracked as #504 (split\nparticipant-set views),\n#505 (malformed peer artifact fail-closes the receiver), #506\n(cross-platform runtime coverage),\nand #507 (per-advance performance follow-ups). `PLAN.md` queues #504.\n\n## Tests\n\n- `fallback_takes_over_after_the_coordinator_goes_silent`: N=5, 100 ms\nlinks, coordinator heard\nthen isolated; the reachable majority must certify the target before the\n5 s socket timeout and\n  keep confirming. Red on the old latch.\n- `fallback_receives_a_relayed_prepare_through_a_broken_direct_link`:\ncoordinator isolated,\nrequester->fallback direct link broken; the relay must forward the first\ncopy so the fallback\n  takes over on time.\n- `fallback_coordinator_commits_through_rough_loss`: same topology over\n2-10% loss, delay,\n  jitter, duplication, and loss bursts.\n- Unit guards for silence-based adoption/blocking plus direct-commit and\nCommitted-report\nstaging before the local cut, including\n`evicted_coordinator_journal_cannot_release_an_heard_active_attempt`.\n- The PCT lagging-survivor premise vector was updated to its new\ndeterministic value (the\ncontrol-path change shifted the interleaving by a few frames while\npreserving the shape).\n\n## Verification\n\n- `cargo fmt`, `cargo clippy --workspace --all-targets --features\ntokio,json`: clean.\n- Full suite: 3663 passed / 77 skipped.\n- Hot-join feature suite: see CI.\n- Targeted fallback/commit tests: 21 passed.\n- cargo-mutants over the fallback corpus: 23 caught, 4 unviable, 0\nmissed.\n- `check-plan-hygiene.py`, `markdownlint`, `typos`, changelog/version\nchecks: clean.\n- Cross-target `cargo check` for wasm32-unknown-unknown / wasi /\nemscripten, i686, aarch64.\n\n## Type of Change\n\n- [x] Bug fix (non-breaking change which fixes an issue)\n\n## Checklist\n\n- [x] I have read the CONTRIBUTING guide\n- [x] I have followed the zero-panic policy\n- [x] I have added tests that prove the fix is effective\n- [x] `cargo fmt` and clippy are clean\n- [x] Full suite passes",
          "timestamp": "2026-11-20T06:15:08Z",
          "tree_id": "bd583ba3ca7ced2717ad1271cf4471cd6c8ebb65",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/c7b3f7b1e9bd3b0fa5a1be42a0bc46040b9b9201"
        },
        "date": 1795156072767,
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
            "value": 24,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 47,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 123,
            "range": "± 3",
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
            "value": 78,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 101,
            "range": "± 2",
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
            "value": 145,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 204,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 297,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 251,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 361,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 521,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 162,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 154,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 211,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 279,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 333,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 435,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 486,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 580,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 786,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 384,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 481,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 649,
            "range": "± 73",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 793,
            "range": "± 8",
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
            "value": 133,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 443,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 697,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1000,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 34,
            "range": "± 2",
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 63,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 237,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1110,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 49802,
            "range": "± 444",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 241256,
            "range": "± 3414",
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
          "id": "e9ca590788b03bb312bbc6c808a048dcf008c72f",
          "message": "fix: bind coordinated-drop participants to the agreed membership epoch (#509)\n\n## Description\n\nCloses #504, #505, #506, and #507, and carries the three unresolved\nreview\nfindings from merged PR #508.\n\n### 1. Split-view participant sets (#504)\n\n`CoordinatedDropState` now derives the participant set from the **agreed\nmembership epoch** (every configured `Local`/`Remote` slot that has not\nbeen\ncertified dropped; reserved hot-join slots only while a\nserve/reactivation is\nin flight), independent of local endpoint reachability. A\nparticipant-set\ndigest is bound into `DropOperationId`. Two disjoint split-view groups\ncan no\nlonger each hold a strict-majority certificate for one target:\n\n- The epoch uses one canonical voter per endpoint (the lowest local\nhandle;\nthe lowest handle of each remote endpoint) and excludes\ncertified-dropped\n  slots plus slots whose serve/reactivation is still in flight.\n- `StateSnapshot` now carries the era's certified-dropped handle list,\nso a\nfresh N-peer joiner derives the same participant set as existing\nsurvivors;\n  the lifecycle commit clears the exclusion on every peer.\n- `begin_coordinated_drop`, deferred-prepare revalidation, and prepare\n  validation all use the same epoch-derived set.\n- The automatic-drop admission denominator keeps the target slot, so a\nsymmetric 2/2 split cannot become 2/3, and the target's endpoint never\ncounts\n  as one of its own survivors.\n- The removal queue records whether an intent came from the automatic\ntimeout\npath: an explicit request queued behind another operation is not gated\nby the\nautomatic survivor-majority pre-check, and an intent whose target\nalready\n  committed is dropped instead of minting a redundant operation.\n- Operation ordering and the closed-commit-cut watermark include the\n  participant digest.\n- Protocol version is now 4 (`wire_golden_v4.rs` is the active suite; v3\nis a\nrejection suite). The `DropOperationId` wire layout grows by eight\nbytes.\n- `CoordinatorSplitView.tla` models two disjoint groups: the shipped\nepoch\npolicy proves `AtMostOneCommittedCut` and liveness non-vacuously\n(GROUP_B 5\ncommits, GROUP_F 3 cannot); the local-view cfg is a manual demo-fail\n(not\n  auto-run).\n- Deterministic two-requester split-view CAP row plus\nparticipant-derivation\n  and identity-binding unit guards.\n\n**CAP trade-off:** in a `Running` session an explicit removal from a\nminority\nof a larger membership now fails closed; a two-player session keeps its\none-survivor certificate. `Synchronizing` sessions retain the documented\nlocal-reachable behavior because no confirmed history can fork yet. An\nexplicit\nremoval racing an in-flight join now excludes the joining slot and\ncommits a\none-survivor certificate; the N-peer survivor-drop battery pins that the\ncoordinator continues and the joiner activates.\n\n### 2. Peer artifacts no longer halt the receiver (#505)\n\nMalformed/stale prepares, malformed inventory/donor reports, conflicting\ncut/digest claims from non-authority participants, and malformed or\nduplicate\nbackfill chunks are discarded with a `NetworkProtocol` violation\ndiagnostic.\nA deterministic coordinator or acting fallback that conflicts with a\nlocked\ncut/digest still fails closed, as do local invariant/allocation\nfailures,\nconflicts with a locally committed certificate, and failed local\nbackfill\ninstalls. Backfill grid locks are installed only after all shape checks\npass.\n\n### 3. Per-advance performance (#507)\n\n- `UdpProtocol` carries a monotone `peer_connect_status_has_disconnect`\nlatch\nset by gossiped status merges, test seeds, and the joiner-bootstrap\nseeder.\n- `update_player_disconnects` skips the full-mesh fold while every local\nstatus\nis connected, no running non-reserved endpoint latched a disconnect, no\n  hot-join attempt is in flight, and no coordinated-drop state exists.\n- `on_input` classifies the piggybacked ack once and applies it through\nthe\nnew `apply_classified_ack_frame`; `apply_ack_frame` keeps the\nclassifying\nwrapper. A test-only counter pins one classification per accepted\npacket.\n\nBenchmark (`cargo bench --bench h16p_mesh -- 'H-16P advance_frame'` vs\nmain):\nN=8 32.6µs (-14.6%), N=16 155.4µs (-2.2%). The healthy early-out also\nstays\ndisabled while any non-reserved remote endpoint is stopped.\n\n### 4. Cross-platform runtime coverage (#506)\n\n- `codec::wire_codec_digest()` encodes/decodes every coordinated-drop\nbody\nshape plus a sync request. The replay fixture pins the digest; a native\nfixture test, the browser smoke test, and the Godot/Emscripten probe\nassert\nit. The Godot `MemorySocket` now transfers encoded bytes and decodes on\n  receive, so the codec runs under Emscripten.\n- The i686 and aarch64 cross-arch lanes now run the feature-gated\nhot-join\n  session suite in addition to the simulation trace.\n- A new `wasip1-replay-probe` binary runs machine-width parity,\ncompression\nparity, wire-codec digest, and the 4,096-frame replay under wasmtime in\nthe\n  `wasm32-wasip1` lane.\n- Platform notes in `docs/specs/determinism-model.md` corrected.\n\n### 5. PR #508 review follow-ups\n\n- A fallback cut announcement that matches an already-staged commit cut\nis\naccepted and adopts the acting role; a genuinely different cut from the\n  authority still fails closed.\n- The rough-loss row installs an explicit Rough `LinkPolicy` on every\ndirected\nlink (it previously used clean links because hand-built schedules do not\n  sample `config.noise`) and runs 1,800 steps; a 4.5s prefix asserts the\n  removal commits before the five-second socket timeout.\n- Former-joiner seam tests no longer use a quorum-less `remove_player`\nagainst\n  an unpolled coordinator endpoint.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] 💥 Breaking change (wire protocol v4 + explicit-removal CAP\nbehavior)\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n- [x] I have followed the zero-panic policy\n- [x] I have added tests that prove the fixes are effective\n- [x] `cargo fmt` and `cargo clippy --workspace --all-targets --features\ntokio,json,hot-join` are clean\n- [x] `cargo nextest run --workspace --all-targets --features\ntokio,json,hot-join,violation-census`: 3746 passed, 77 skipped\n- [x] Documentation and `CHANGELOG.md` updated (breaking wire/protocol +\nCAP behavior)\n- [x] `agent-preflight.py --full --auto-fix` passes\n- [x] Cross-target `cargo check` for wasm32-unknown-unknown / wasip1 /\nemscripten, i686, aarch64\n- [x] TLA `CoordinatorSplitView` passes; local-view cfg is an enforced\ndemo-fail\n\n## Related Issues\n\nCloses #504\nCloses #505\nCloses #506\nCloses #507\nAddresses the unresolved review threads on #508",
          "timestamp": "2026-11-20T19:56:16Z",
          "tree_id": "2e69a57f7aa46867e8746f640410ed69cbcf1616",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/e9ca590788b03bb312bbc6c808a048dcf008c72f"
        },
        "date": 1795205306472,
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
            "value": 23,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 85,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 111,
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 22,
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
            "value": 55,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 73,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 98,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 87,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 129,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 204,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 152,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 243,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 386,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 95,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 99,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 141,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 156,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 198,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 274,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 267,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 342,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 519,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 249,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 323,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 464,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 598,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 83,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 110,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 349,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 596,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 818,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 39,
            "range": "± 10",
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
            "value": 16,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 196,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 820,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 36536,
            "range": "± 198",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 179064,
            "range": "± 779",
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
          "id": "0085c493f81796124f6590261c153923be4809b8",
          "message": "test: close the #509 mutation survivors in the new drop/hot-join paths (#511)\n\n## Description\n\nThe post-merge `main` mutation job for PR #509 reported **44 missed\nmutants**\nin the new coordinated-drop split-view, peer-artifact, hot-join\nexclusion, and\ndisconnect-fold code. This follow-up closes every survivor without\nchanging\nbehavior:\n\n- Extract the boolean membership/authority predicates into pure helpers\n  (`hot_join_in_flight`, `report_has_authority`, `source_has_authority`,\n  `timeout_drop_is_automatic`, `status_table_is_clean`,\n`endpoint_requires_disconnect_fold`, `no_hot_join_in_flight`) and cover\nevery\n  branch with unit tests.\n- Reuse one `certified_dropped_list_is_valid` validator in both the\n2-peer and\n  N-peer snapshot paths; test bounded, sorted, and in-range cases.\n- Add a test-only `disconnect_fold_runs` counter so the healthy\nearly-out and\neach dirty view (gossip, stopped endpoint, truncated status table, local\n  disconnect) are observable.\n- Add targeted tests for: queue origin (explicit vs automatic, including\na\nreserved-slot automatic intent), unbound participant digests, zero-frame\nbackfill chunks, coordinator/fallback cut conflicts after a fallback\nlock,\n  authority-scoped Ready/Committed report conflicts, the relay of newly\n  accepted reports, and the per-body decode-error prefixes in\n  `decode_drop_operation_id`.\n\n## Verification\n\n- Focused `cargo-mutants --in-diff` over the branch diff vs `main`: **57\n  mutants, 32 caught, 25 unviable, 0 missed, 0 timeout**.\n- Full workspace nextest (`tokio,json,hot-join,violation-census`): 3760\npassed,\n  77 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json,hot-join\n-- -D\n  warnings`: clean.\n- `agent-preflight.py --full --auto-fix`: all checks pass.\n\n## Type of Change\n\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n- [x] I have followed the zero-panic policy\n- [x] I have added tests that prove the fixes are effective\n- [x] `cargo fmt` and clippy are clean\n- [x] Full suite passes\n\n## Related Issues\n\nFollow-up to #509; addresses the post-merge mutation-testing failure on\n`main` (`e9ca5907`).",
          "timestamp": "2026-11-20T21:56:34Z",
          "tree_id": "fb7e66377080df78524abe1c3f2621d5c0e3c109",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/0085c493f81796124f6590261c153923be4809b8"
        },
        "date": 1795212462291,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 85,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 39,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 112,
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
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 22,
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
            "value": 54,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 97,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 90,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 126,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 193,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 152,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 229,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 374,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 101,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 131,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 145,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 182,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 252,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 268,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 338,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 495,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 247,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 329,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 463,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 595,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 84,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 110,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 358,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 577,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 799,
            "range": "± 65",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 32,
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
            "value": 15,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 50,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 192,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 754,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 37080,
            "range": "± 142",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 178109,
            "range": "± 935",
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
          "id": "eaad20973a6eb8946b7a56c53a1750a4ce7d4b90",
          "message": "ci: keep the cross-arch default suite free of hot-join (#513)\n\n## Description\n\nRestores green `main` after the post-merge `CI - Rust` failure on\n`0085c493`. The only failed check was **Cross-Arch Runtime\n(aarch64-qemu)**, step **Run deterministic suite** (`exit code 100`,\n7m03s); the two dedicated hot-join steps never ran. Run:\nhttps://github.com/Ambiguous-Interactive/fortress-rollback/actions/runs/44825116296/job/136595656123\n\n### Root cause\n\nThe step ran `cargo nextest ... --workspace --features tokio,json`, but\n`network-test-peer` unconditionally enables\n`fortress-rollback/hot-join`. Cargo feature unification therefore built\nthe root package **with hot-join**, so the suite that the workflow\ncomment says \"compiles it out\" actually ran 3,246 tests including\n`network::soak::*` and `sessions::hot_join::*`.\n\nTwo consequences in the same step:\n\n- `sessions::hot_join::hot_join_phase4_abort_does_not_spam_send_queue`\ndrives 120,000 polls and takes **73.2s** under `qemu-aarch64` on the\ndevelopment host. The `ci-cross-arch` profile terminates a test after\n240s; a slower CI runner crosses that budget and nextest exits 100.\n- `stats_alloc_realloc_growth_is_already_embedded_in_allocated_bytes`\nreads a process-global `Region`, so its exact `allocations`/net-live\nassertions race with harness allocations on another thread (observed\nunder QEMU).\n\n### Fix\n\n- Scope the deterministic suite to `-p fortress-rollback`, so `hot-join`\nreally is compiled out and the feature-gated rows run only in their\ndedicated steps. The workflow comment now explains the unification\nhazard.\n- Deflake the allocator contract: 1 MiB -> 2 MiB growth, no exact\nprocess-global counter assertions, 64 KiB slack for allocator noise, and\nthe double-count guard still distinguishes 2 MiB from 3 MiB. The\nend-to-end test moved to `tests/stats_alloc_contract.rs`, a single-test\nprocess, so `cargo test` cannot run another test concurrently against\nthe same global allocator. A deterministic alloc/realloc noise thread\npasses 20/20 under qemu-aarch64.\n\n### Regression coverage\n\n- `scripts/tests/test_ci_rust_workflow.py` now requires package scope\nand rejects `--workspace` for that step.\n- A deterministic 4 KiB cross-thread allocation reproduced the soak\nfailure on the old assertions and passes with the fix.\n\n## Type of Change\n\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json,hot-join` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] My changes generate no new compiler warnings\n- [ ] Documentation updated (not user-facing)\n- [ ] CHANGELOG entry (test/CI only, no `pub` or user-observable change)\n\n## Testing\n\n**Tests added/modified:**\n\n- `test_cross_arch_runtime_executes_arm_and_32_bit_usize_targets` now\npins `-p fortress-rollback` and the absence of `--workspace`.\n-\n`tests/stats_alloc_contract.rs::stats_alloc_realloc_growth_is_already_embedded_in_allocated_bytes`\ntolerates harness noise while keeping the growth-embedding and\ndouble-count bounds; it runs as the only test in its process.\n\n**Manual testing performed:**\n\n| Lane | Command | Result |\n| --- | --- | --- |\n| aarch64 QEMU (new command) | exact CI filter, `-p fortress-rollback` |\n**2,945 passed, 503 skipped, 166.5s** |\n| i686 (new command) | exact CI filter, `-p fortress-rollback` | **2,943\npassed, 503 skipped, 7.9s** |\n| host workspace | `tokio,json,hot-join,violation-census` | **3,692\npassed, 77 skipped, 39.3s** |\n| static | `actionlint`, `pytest scripts/tests/test_ci_rust_workflow.py`\n(31), `agent-preflight.py --full --auto-fix` | pass |\n\nRed/green for the soak test with deterministic helper-thread noise:\nbaseline `allocations: 12 != 1` (exit 101); the current test passes\n20/20 with an alloc + realloc + promote noise thread under qemu-aarch64\n(exit 0).\n\n## Related Issues\n\n- Relates to #512 (coordinated-drop terminal-operation eviction; filed\nas a follow-up finding).\n- Review follow-up: the three unresolved #508 threads were verified\nagainst `e9ca5907` and resolved with pointers to the fixing tests.",
          "timestamp": "2026-11-20T23:56:12Z",
          "tree_id": "a8508bb00bddb4d1816e1e168a33d51d2cca52e0",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/eaad20973a6eb8946b7a56c53a1750a4ce7d4b90"
        },
        "date": 1795219677065,
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 37,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 119,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 43,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 129,
            "range": "± 1",
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 75,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 103,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 130,
            "range": "± 4",
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
            "value": 178,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 262,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 224,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 343,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 515,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 124,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 133,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 165,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 183,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 236,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 321,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 365,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 435,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 612,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 284,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 386,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 546,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 687,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 108,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 591,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 1007,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1519,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 33,
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
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 67,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 213,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 844,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 44935,
            "range": "± 152",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 205691,
            "range": "± 5303",
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
          "id": "0819ad574bcb7a67811994bdf29ccdd1f7ce9585",
          "message": "fix: keep evicted terminal coordinated-drop operations closed (#514)\n\n## Description\n\nCloses #512.\n\n`coordinated_drop_close_operation` kept terminal operations in a\n`closed` deque bounded to `num_players`. Once an aborted or superseded\noperation was evicted, a delayed `Prepare` could still pass\n`coordinated_drop_validate_prepare` when the target was connected, its\ngeneration unchanged, and the membership epoch unchanged. With no active\nattempt, the endpoint installed a new `Inventory` and broadcast a\n`Report`; peers that still remembered the operation as closed did not\nanswer, the reopened attempt hit its deadline, and\n`coordinated_drop_fail_closed` took a healthy session into the\ndisconnect fail-closed state.\n\n`DropOperationId.sequence` is generation-local per target set\n(`max(target generation) + 1`), not a coordinator-global counter, so the\nguard keys on the full operation identity.\n\n### Fix\n\n- Require a canonical target set (every remote handle of the target\nendpoint) and a canonical operation coordinator (the first participant).\nThis rejects non-canonical subsets and forged authority, and means only\none identity per target handle/generation can validate at a time.\n- Record it from every terminal close and from the supersede/reject\npath.\n- Keep a per-target-handle history of **exact** terminal identities\n`(target_set_digest, participant_set_digest, coordinator_generation,\ntarget_generation, sequence)`, bounded to `2 * num_players` per handle.\nExact matching is required: a lower sequence can still be a live\nhigher-priority attempt, and a rejoined coordinator starts a new\ngeneration. A different live sequence is left to the operation\ncomparator. A replay older than the bounded window is tracked in #516\nfor a protocol-level terminal epoch.\n- Skip exact terminal identities and closed-journal identities one step\nat a time in `begin_coordinated_drop`; a terminal identity at a large or\nhostile sequence does not move the retry floor, so it cannot exhaust the\nsequence.\n- Share the operation-identity tuple helper with `closed_commit_cuts`;\nthe target journal is bounded to one entry per player handle.\n- Fold the behavior into the existing coordinated-drop artifact\nchangelog entry (unreleased code rule).\n\n### Red/green\n\nNew regression\n`evicted_terminal_prepare_does_not_reopen_and_retry_advances`:\n\n```text\npanicked at src/sessions/p2p_session.rs:19012:9:\nan evicted terminal prepare must not reopen an inventory attempt\ntest result: FAILED. 0 passed; 1 failed\n```\n\nAfter the fix the same test passes. Additional regressions cover the\nrejected-journal eviction replay, a hostile large terminal identity, a\nclosed identity above the terminal one, a rejoined coordinator\ngeneration, and exact-identity/bound scoping.\n\n### Known limitation\n\nThe exact-identity history is bounded to `2 * num_players` entries per\ntarget\nhandle by the repository's bounded-allocation policy. An exact replay\nolder\nthan that window can still reopen; closing that requires a\nprotocol-level\nterminal epoch (issue #516). The changelog and the code-side bound note\nstate\nthis explicitly. Canonical target-set/coordinator validation limits\nwhich\nidentities can be valid, and the exact-match guard keeps a different\nlive\nsequence admissible.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json,hot-join` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] CHANGELOG entry folded into the existing unreleased\ncoordinated-drop entry\n- [x] My changes generate no new compiler warnings\n- [ ] Documentation updated (internal protocol dedup; no public API\nchange)\n\n## Testing\n\n**Tests added/modified:**\n\n- `evicted_terminal_prepare_does_not_reopen_and_retry_advances`\n- `evicted_rejected_prepare_still_cannot_reopen`\n- `large_terminal_identity_does_not_block_a_new_retry`\n- `rejoined_coordinator_generation_is_not_blocked_by_terminal_history`\n- `terminal_watermark_matches_exact_identity_and_is_bounded`\n- `retry_steps_once_past_a_closed_identity_after_the_terminal_watermark`\n- `evicted_terminal_prepare_does_not_reopen_and_retry_advances`\n- `evicted_rejected_prepare_still_cannot_reopen`\n- `retry_steps_once_past_a_closed_identity_after_the_terminal_watermark`\n- `non_canonical_target_subset_does_not_match_membership`\n- `forged_operation_coordinator_is_rejected`\n\n**Manual testing performed:**\n\n| Check | Result |\n| --- | --- |\n| Full workspace (`tokio,json,hot-join,violation-census`) | **3,700\npassed, 77 skipped, 48.1s** |\n| Scoped aarch64 QEMU (exact CI command) | **2,953 passed, 503 skipped,\n179.7s** |\n| Scoped i686 (exact CI command) | **2,951 passed, 503 skipped, 12.6s**\n|\n| `cargo clippy --workspace --all-targets --features tokio,json,hot-join\n-- -D warnings` | clean |\n| `cargo test --doc` | 160 passed, 51 ignored |\n| `agent-preflight.py --full --auto-fix` | all checks pass |\n| Targeted mutation of the changed helpers | record helper 14 mutants\n(12 caught, 2 unviable); exact-match/admission set **0 missed** after\nthe duplicate-identity regression |\n\nReview follow-ups: the initial per-identity journal could evict an\noperation; the final guard is an exact per-handle identity history.\nLater rounds found target-set sequence mixing, a rejoined coordinator\ngeneration, a hostile large terminal sequence, and a prefix rule that\nrejected live lower sequences; all are fixed by exact matching,\ncoordinator-generation identity, canonical target/coordinator\nvalidation, and no sequence floor. The bounded-window residual is\ntracked in #516.\n\n## Related Issues\n\n- Fixes #512\n- Relates to #431 (original review finding) and #510 (CAP trade-off\ntriage).",
          "timestamp": "2026-11-21T04:35:52Z",
          "tree_id": "ea3d0f0791afaf24204c9f566afa86b418b78fcc",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/0819ad574bcb7a67811994bdf29ccdd1f7ce9585"
        },
        "date": 1795236558617,
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
            "value": 24,
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 39,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 107,
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
            "value": 29,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 47,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 124,
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
            "value": 78,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 95,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 127,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 143,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 199,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 359,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 255,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 360,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 548,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 143,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 246,
            "range": "± 52",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 211,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 276,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 330,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 420,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 479,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 573,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 765,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 331,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 453,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 645,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 812,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 96,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 134,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 432,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 713,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1005,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 42,
            "range": "± 1",
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 65,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 244,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 970,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 50472,
            "range": "± 231",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 234637,
            "range": "± 1152",
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
          "id": "caa387b1901c310719f99954b3897212dbd524b7",
          "message": "test: exercise certified-dropped hot-join snapshots in wasm (#517)\n\n## Description\n\nCloses #515.\n\nAdds a runtime 3-peer probe to both wasm lanes: the host owns a live\nremote and a reserved hot-join slot, removes the remote, drains the\nremoved target's speculative tail, then serves the reserved slot. The\njoiner must import the snapshot's non-empty `certified_dropped` list and\nderive the epoch `[0, 2]`, not `[0, 1, 2]`.\n\n### Production bug exposed by the scenario\n\nThe first run failed the host removal with:\n\n```text\ncoordinated graceful drop ... failed closed: ParticipantLost;\nphase=Inventory, report_count=1\nhost epoch = [0, 1, 2]\n```\n\n`coordinated_drop_membership_epoch` counted a **never-joined reserved\nhot-join slot** as a voter. The reserved slot cannot report, so the\nremoval waited for a quorum that could never assemble and the healthy\nsession failed closed. The epoch now excludes every reserved slot until\nits join/reactivation commit activates it. Local in-flight state is not\nconsulted, so a peer that has observed a serve cannot derive a different\nquorum from one that has not. The roster and reserved flags are agreed\nconfiguration on every peer, now documented on `add_reserved_player`.\n\nThe scenario also needed two post-removal advances before serving,\nbecause the N-peer serve captures `last_saved_frame` and waits for it to\nconfirm; the removed target can leave a speculative tail. This is app\npacing, not a production bug; the serve gate already fails closed rather\nthan serving an unconfirmed frame.\n\n### Coverage changes\n\n- Browser `RawBus` generalized from two fixed queues to a mesh; `pair()`\nis retained.\n- New browser test\n`hot_join_imports_certified_dropped_epoch_in_browser`, with a scoped\n`CollectingObserver` that prints violations on failure.\n- New Emscripten `MeshSocket` routes wire-encoded bytes between 3\naddresses; `run_certified_epoch_probe` mirrors the flow and `run_probe`\nreports `certified_epoch_imported`; the Playwright spec asserts it.\n- Doc-hidden `P2PSession::diagnostic_coordinated_drop_membership_epoch`\nexposes the derived epoch to the probes; the public API snapshot is\nregenerated.\n- Native regression\n`membership_epoch_excludes_never_joined_reserved_slots`.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json,hot-join` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] CHANGELOG updated (epoch fix folded into the split-view entry; the\ndiagnostic is in Added)\n- [x] Public API snapshot regenerated and checked (`2,769` symbols)\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `membership_epoch_excludes_never_joined_reserved_slots`\n- `hot_join_imports_certified_dropped_epoch_in_browser`\n- `run_certified_epoch_probe` + `certified_epoch_imported` in the\nGodot/Emscripten probe and Playwright spec\n\n**Manual testing performed:**\n\n| Check | Result |\n| --- | --- |\n| Browser suite (Chrome 158 headless, wasm32-unknown-unknown, hot-join)\n| **11 passed, 0 failed** |\n| Native full workspace (`tokio,json,hot-join,violation-census`) |\n**3,701 passed, 77 skipped** |\n| Scoped aarch64 QEMU suite | **2,953 passed, 503 skipped** |\n| Targeted aarch64 hot-join epoch test | 1 passed |\n| Emscripten probe `cargo +nightly-2026-08-26 check` | pass |\n| Emscripten probe clippy (all targets/features, `-D warnings`) | pass |\n| `cargo clippy --workspace --all-targets --features tokio,json,hot-join\n-- -D warnings` | clean |\n| `cargo test --doc` | 160 passed, 51 ignored |\n| `public_api_census.py --check` | 2,769 symbols match |\n| `agent-preflight.py --full --auto-fix` | all checks pass |\n\n## Related Issues\n\n- Fixes #515\n- Relates to #516 (bounded terminal-history epoch design)",
          "timestamp": "2026-11-21T05:48:52Z",
          "tree_id": "5d397e04d0c09c9d5ccfadfa7982d676477b0c7c",
          "url": "https://github.com/Ambiguous-Interactive/fortress-rollback/commit/caa387b1901c310719f99954b3897212dbd524b7"
        },
        "date": 1795240926218,
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
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 19,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 104,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 45,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 149,
            "range": "± 0",
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
            "value": 22,
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
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 71,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 81,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 110,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 102,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 144,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 220,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 189,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 268,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 469,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 93,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 112,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 147,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 192,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 242,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 314,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 325,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 405,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 558,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 238,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 337,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 509,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 699,
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
            "value": 117,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 552,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 924,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1374,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 23,
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
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 63,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 223,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 871,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=8",
            "value": 45267,
            "range": "± 729",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P advance_frame/warmed_mesh/N=16",
            "value": 214207,
            "range": "± 1314",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}