window.BENCHMARK_DATA = {
  "lastUpdate": 1794969827538,
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
          "id": "1af6062c20f76662830344655c12a8f714a6de84",
          "message": "test(simulation): prove graceful removal equivalence (#375)\n\n## What\n\nExtends the bounded two-peer differential rollback oracle to one\nexplicit graceful removal. The independent reference now models the\ncertified freeze cut, including exact input and `StateStub` boundaries,\nrather than relying only on peer agreement.\n\nAlso keeps session evidence in the local, gitignored progress log and\nnarrows the active P0 item in `PLAN.md` to the remaining membership\nscenarios. Per review, all previously tracked progress logs are removed\nfrom git and canonical agent guidance now explicitly keeps them\nmachine-local.\n\n## Why\n\nA mesh can agree on the same wrong frozen input or state. This change\nadvances the active rollback-equivalence milestone by checking the\nsurviving fixed-membership trajectory against an independent nominal\nhistory and by proving that abandoned pre-removal observations cannot\ncreate false divergence.\n\n## Reference semantics\n\nFor certified cut `F`:\n\n- Input `F` remains nominal; frames after `F` repeat that value for the\nremoved target.\n- Post-state `F+1` consumes nominal input `F` and remains\nboundary-valid.\n- Post-state `F+2` is the first state that can observe frozen input.\n- The departed peer's required `F+1` state remains checked when\nconfirmed and simulated; its abandoned `F+2+` generation is excluded\nfrom reference and legacy cross-peer state comparisons.\n- Target-authored confirmed inputs above `F` cannot define the\nsurvivor's frozen continuation.\n\nThe two-peer commit guard requires synchronous disconnection, a Running\nsurvivor, a cut equal to the pre-call direct receipt, and pre-call\nsurvivor confirmed/sampled history bounded by the cut.\n\n## Coverage and controls\n\n- EveryFrame/delay-0/reordered and Sparse/delay-2/delayed schedules\n- Forward, reverse, and rotating drive orders\n- Exact trace, trajectory, control, and failure-vector artifact replay\n- Matched no-removal input/state controls and post-removal\nrollback/load/resimulation evidence\n- Frozen-input, wrong-state, missing-state, and final-frame mutation\ncontrols\n- Explicit non-vacuity evidence for target-authored inputs above `F` and\nabandoned target states at `F+2+`\n\nThe claim remains bounded to two peers and Lockstep cadence. Automatic\ndisconnects/timeouts, spectator membership changes, hot-join, and the\nother active P0 scenarios remain in `PLAN.md`.\n\n## External reconciliation\n\n- Only open issue: #365, intentionally P2 behind the active correctness\nwork\n- No pre-existing open, draft, or dependency PRs\n- No compatible Cargo or npm updates; fuzz, Loom, and Godot standalone\nworkspaces are current\n- Main CodeQL push cancellation was caused by replacement with the\nscheduled run at the same SHA; that scheduled run completed\nsuccessfully, so no source/workflow fix was needed\n\n## Validation\n\n- Focused differential selection: 32 passed\n- Full simulation target: 399 passed, 26 skipped\n- Full Nextest: 3,029 passed, 73 skipped\n- `cargo clippy --workspace --all-targets --features tokio,json`: passed\nwithout warnings\n- Rustdoc: passed; documentation tests: 160 passed, 50 ignored\n- Agent preflight, formatting, diff checks, plan hygiene, typos,\nsemantic doc claims, and 1,442 documentation links: passed\n- All four workspace lock checks, `cargo audit`, `cargo deny check`, and\n`npm audit`: passed\n- Independent adversarial review iterated through all findings; final\nre-review reported zero issues\n\nNo production or public API code changed, so no changelog entry is\nrequired.\n\n## Review readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS (no production diff)\n- Determinism: PASS (seeded schedules and ordered histories)\n- Error handling: PASS (structured oracle failures and explicit commit\nguard)\n- Test breadth: PASS (nominal matrix, exact replay, boundaries, and\nnegative controls)\n- Agent preflight: PASS\n- Design log reviewed: N/A (test-only oracle extension)\n- Changelog reviewed: N/A (no user-observable/public change)",
          "timestamp": "2026-11-07T22:55:19-08:00",
          "tree_id": "1560cc841c7610a370172436b9a3217ba5c286c2",
          "url": "https://github.com/wallstop/fortress-rollback/commit/1af6062c20f76662830344655c12a8f714a6de84"
        },
        "date": 1794121403898,
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
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 81,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 274,
            "range": "± 1",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 57,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 152,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 522,
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
            "value": 98,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 128,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 166,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 179,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 253,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 394,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 341,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 467,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 702,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 174,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 221,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 248,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 346,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 414,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 538,
            "range": "± 6",
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
            "value": 768,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1041,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 380,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 526,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 688,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 921,
            "range": "± 9",
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
            "value": 136,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 400,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 686,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 946,
            "range": "± 27",
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 86,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 324,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1493,
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
          "id": "5403150f84175c4271a92521b5f4a1cb7ee9bc81",
          "message": "test(simulation): prove bounded membership equivalence (#376)\n\n## What\n\nAdvances the active differential rollback-equivalence milestone along\nall bounded membership paths that are currently testable, plus\ncompatible tool updates:\n\n- Three-peer automatic disconnect/timeout (`ContinueWithout`) with an\nindependently certified freeze cut.\n- Spectator-visible explicit removal with exact frozen fingerprints and\n`Disconnected` statuses.\n- Bounded two-peer hot-join with a certified removal cut, snapshot\nactivation, replacement state floor, and authoritative activation-frame\nrewrite.\n- Loom 0.7.3 and MkDocs Material 9.8.1 test/docs updates.\n\n## Why\n\nA mesh can agree on the same wrong frozen input or state. These\nscenarios compare the confirmed trajectory against an independent\nnominal model and prove their boundaries with mutation controls, rather\nthan treating peer agreement as correctness.\n\nTwo-peer automatic timeouts intentionally fail closed for lack of an\nindependent live witness, so the timeout scenario uses three real\nsessions. Spectator equivalence is restricted to the surviving host\nbecause a removed player's spectator endpoint is not withdrawn by\n`remove_player`. Hot-join remains bounded to the currently proven\ntwo-peer/forward shape.\n\n## Coverage\n\n### Automatic disconnect\n\n- Actual post-poll certified cut `F`; both survivors must agree on\ntarget and cut.\n- Pre-poll confirmed/sampled history bounded by `F`.\n- EveryFrame/delay-0/reordered and Sparse/delay-2/delayed schedules\nacross forward, reverse, and rotating orders.\n- Exact target-specific interruption/drop/disconnect events, >20\npost-cut confirmed frames, post-commit rollback/resimulation/load, exact\nreplay, and artifact roundtrip.\n- Shared frozen-input, wrong-state, and missing-state mutation controls.\n\n### Spectator removal\n\n- Frame `F` nominal and `Confirmed`; frames `F+1..` frozen and\n`Disconnected` for the target only.\n- Matrix across both removal rows and all three drive orders, with\nmatched no-removal baselines.\n- Exact replay and survivor-plus-spectator cross-order projection.\n- Shared frozen-input lie remains visible in the spectator stream;\nspectator-only status lie at `F+1` is rejected.\n\n### Hot-join\n\n- Freeze `F < k < A` at the removed generation's cut input; activation\nframe `A` is rewritten to the replacement's nominal input; recorded\nreplacement states begin at `A+1`.\n- The activation frame is sampled as soon as confirmed and retried until\nthe rewrite is visible, so long drains cannot evict it or retain a stale\nfreeze.\n- Held-handshake row forces a multi-frame freeze window; exact replay,\n400-step long-drain check, mutation controls, replacement-floor\ncontrols, and artifact roundtrip.\n- Validation bounds the scenario to two peers, slot 1, zero delay,\nEveryFrame, clean links, one replacement, and no composed\nlifecycle/spectator events.\n\nRemaining N-peer hot-join and redundant-spectator failover are tracked\nin [#377](https://github.com/wallstop/fortress-rollback/issues/377) and\n`PLAN.md`.\n\n## External and dependency reconciliation\n\n- Paginated open issues: only #365, intentionally P2 behind correctness\nwork.\n- No pre-existing open or draft PRs. The stale parking_lot Dependabot\nbranch was already merged and is now MSRV-incompatible.\n- Latest `main` SHA `1af6062c20f76662830344655c12a8f714a6de84`: 90\nsuccessful checks, five intentional skips, no failures.\n- Loom 0.7.3 updated in all four workspace locks. Other newer direct\nCargo releases retain their documented MSRV holds.\n\n## Validation\n\n- Differential suites: 36 default, 39 with `hot-join`, passed.\n- Full simulation: 404 default, 417 with `hot-join`, passed (26\nskipped).\n- Full Nextest: 3,034 default and 3,295 with `hot-join`, passed (73/74\nskipped).\n- Loom 0.7.3 release tests: 19 passed.\n- Strict Clippy all workspace/targets with `tokio,json` and with\n`hot-join,tokio,json`: passed without warnings.\n- Rust 1.86 `cargo check --locked --all-targets`: passed.\n- Rustdoc: passed; documentation tests: 160 passed, 50 ignored.\n- MkDocs build with Material 9.8.1: passed.\n- Agent preflight, formatting, diff checks, plan hygiene, typos,\nsemantic doc claims, and documentation links: passed.\n- All four workspace lock checks, Cargo audit, Cargo deny, and npm\naudit: passed.\n- Independent adversarial review iterated through all findings; final\nreviews reported no behavioral issues.\n\nNo production or public API code changed, so no changelog entry is\nrequired.\n\n## Review readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS (test-only oracle changes)\n- Determinism: PASS (seeded virtual-time schedules, exact replay\neverywhere)\n- Error handling: PASS (typed oracle failures and bounded validation)\n- Test breadth: PASS (nominal matrices, boundaries, event identity,\nexact replay, negative controls)\n- Agent preflight: PASS\n- Design log reviewed: N/A (test-only oracle extension)\n- Changelog reviewed: N/A (no user-observable/public change)",
          "timestamp": "2026-11-09T11:50:15-08:00",
          "tree_id": "77303b6e64e946fe1fa7589bcdff4ba121d6f7c7",
          "url": "https://github.com/wallstop/fortress-rollback/commit/5403150f84175c4271a92521b5f4a1cb7ee9bc81"
        },
        "date": 1794254245561,
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
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 19,
            "range": "± 1",
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
            "value": 51,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 207,
            "range": "± 9",
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
            "value": 20,
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
            "value": 71,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 275,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 16,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 58,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 75,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 85,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 109,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 161,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 170,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 228,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 333,
            "range": "± 3",
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
            "value": 98,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 132,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 185,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 215,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 264,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 337,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 378,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 473,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 257,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 274,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 363,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 474,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 60,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 80,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 372,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 637,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 925,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 7,
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
            "value": 13,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 171,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 874,
            "range": "± 13",
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
          "id": "ede612d6a1988fad6f60dedbbc633e4799e4251b",
          "message": "test(simulation): extend differential oracle to N-peer membership (#378)\n\n## Description\n\nAdvances the active P0 differential rollback-equivalence milestone\n(#377) by widening the bounded fixed-history oracle to N-peer\nmembership, spectator host failover, and queue-capacity boundaries.\n\n### Changes\n\n- **N-peer hot-join (n = 2 and 3).** The harness now models production's\nasynchronous all-survivor removal certificate: the replacement stays\noffline, the old generation is frozen out of the drive loop, the\ncertified cut is anchored when the coordinator observes the commit, and\nreference sampling is suspended until the cut is known. The independent\nreference is width-aware: a two-peer reactivation rewrites snapshot `A`,\nwhile an N-peer serve keeps `A` as a frozen bridge and activates at `A +\n1`. Activation validation now covers every survivor, and exact\ncoordinator/survivor event payloads are pinned.\n- **Three-peer explicit removal + redundant spectators.**\n`GracefulRemove` now supports asynchronous certificates on three-peer\nmeshes. Spectator rows cover host sets `[0]`, `[2]`, `[0, 2]`, and `[2,\n0]`, validating exact frozen fingerprints/statuses, host survival, exact\nreplay, and the certified cross-order projection. The removed player's\nown endpoint is explicitly rejected as a spectator source because\nproduction does not withdraw it and its stale stream correctly fails\nclosed.\n- **Spectator host silence and failover.** Schedule `Block`/`Hold` may\ntarget the spectator endpoint, isolating one host's observer feed.\nPost-removal rows silence the canonical and lower-priority hosts with a\ntimeout above the notify delay and below the outage length, proving the\nsurviving host advances the display, the blocked endpoint is observed as\nfailed, and the exact frozen history resumes with replay identity.\n- **Input-queue capacity boundaries.** New rows fill the queue from the\ndelay side (`delay 120 + prediction 7`) and the prediction side (`delay\n2 + prediction 125`, `MAX_FRAME_DELAY = 127`) and retain the exact\nnominal confirmed history past the boundary.\n- **Cross-order identity.** The bounded hot-join row's activation frame\nis proven order-stable and the complete normalized trajectory is\nidentical across forward/reverse/rotating drive orders.\n- `PLAN.md` is narrowed to the remaining\nhost-crash-as-second-membership-change row and further queue/ring\ncorners.\n\n## Type of Change\n\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy** (no production code\nchanged)\n- [x] I have added tests that prove the coverage is effective\n- [x] `cargo fmt` and strict Clippy (`--workspace --all-targets\n--features tokio,json` and `...,hot-join`) pass with no warnings\n- [x] `cargo nextest run` passes\n\n### If Applicable\n\n- [x] `PLAN.md` updated (test-only change, so no changelog entry)\n\n## Testing\n\n- Differential suites: 37 default and 41 with `hot-join` at the first\ncommit; 38/42 after the host-failover addition, all passing.\n- Full Nextest: 3,036 default and 3,298 with `hot-join`, 73/74\nintentionally skipped.\n- Strict Clippy with `-D warnings` under both feature sets, `cargo fmt`,\nagent preflight, plan hygiene, doc links, typos, and markdownlint pass.\n- Red-green evidence: the N-peer removal initially failed the\nsynchronous-commit premise, then exposed the frozen bridge-frame\nboundary; both were fixed at the harness/reference level and restored\ngreen without production changes. The host-outage row initially failed\nthe builder's notify-delay bound and was corrected to a 600 ms timeout\nabove the 500 ms notify delay.\n\n## Related Issues\n\nRelates to #377. Remaining scope tracked there: a host crash that is\nitself a second membership change, the removed player's own spectator\nendpoint (documented fail-closed), and further queue/ring corners.",
          "timestamp": "2026-11-09T15:23:39-08:00",
          "tree_id": "5eea0a5444b70ce360011f33b9913fd3613744d1",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ede612d6a1988fad6f60dedbbc633e4799e4251b"
        },
        "date": 1794267137717,
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
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 81,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 275,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 33,
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
            "value": 56,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 151,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 522,
            "range": "± 12",
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
            "value": 98,
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
            "value": 167,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 179,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 266,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 382,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 340,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 465,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 691,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 176,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 223,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 246,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 346,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 417,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 533,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 648,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 772,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1015,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 382,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 544,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 701,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 910,
            "range": "± 18",
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 392,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 649,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 917,
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
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=2",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 86,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 327,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1511,
            "range": "± 26",
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
          "id": "6050a99cf07296ca14f5a2318d66b0175c040925",
          "message": "test: warm allocation control before measuring exact sensitivity (#379)\n\n## Description\n\nRCA for the `Build & Test (macos-latest)` failure on main (`ede612d`,\njob 129511543039):\n\n```\nthread 'warmed_hot_paths_obey_allocation_contracts' panicked at tests/allocation_contract.rs:447:\nassertion `left == right` failed: known-allocation control: Stats { allocations: 6, ..., bytes_allocated: 5052, ... }\n  left: 6\n right: 1\n```\n\n`stats_alloc` materializes its per-thread accounting lazily on first\nuse, so the first measured region on that macOS runner was charged six\none-time allocations (956 bytes beyond the known 4 KiB). The same code\npassed on the PR's macOS job minutes earlier, and the\nallocation-contract binary is independent of the simulation-harness\nchanges in that merge.\n\nThe fix charges that one-time setup to a discarded warm-up region before\nthe exact sensitivity control. The control remains exact (one\nallocation, 4,096 bytes, zero reallocations) on every platform, and all\nzero-allocation assertions keep their full teeth.\n\n## Type of Change\n\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n- [x] `cargo fmt` and strict Clippy pass\n- [x] `cargo nextest run -E\n'test(warmed_hot_paths_obey_allocation_contracts)'` passes\n- [x] No production code changed; no changelog entry required\n\n## Testing\n\n- `warmed_hot_paths_obey_allocation_contracts` passes locally.\n- Strict Clippy (`--workspace --all-targets --features tokio,json`)\npasses with `-D warnings`.\n- macOS CI in this PR validates the runner-specific RCA.",
          "timestamp": "2026-11-09T16:01:11-08:00",
          "tree_id": "7c51e6c1d9fbb92bbf9713bfa6e9a5bf2fe2f311",
          "url": "https://github.com/wallstop/fortress-rollback/commit/6050a99cf07296ca14f5a2318d66b0175c040925"
        },
        "date": 1794269340749,
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
            "value": 26,
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
            "value": 232,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 31,
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
            "value": 123,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 448,
            "range": "± 2",
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
            "value": 73,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 91,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 120,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 123,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 165,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 271,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 224,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 315,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 485,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 118,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 133,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 168,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 220,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 260,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 348,
            "range": "± 19",
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
            "value": 508,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 706,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 286,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 369,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 509,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 672,
            "range": "± 5",
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
            "value": 110,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 339,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 544,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 771,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 20,
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 70,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 271,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1107,
            "range": "± 29",
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
          "id": "582180484bc7a95c7c17c7c482c9c1721df45c61",
          "message": "fix(replay): settle recorded checksums after rollback corrections; close #377 membership closure (#381)\n\n## Description\n\nTwo changes, aggregated into one PR:\n\n1. **Fix a production replay-recording bug (#380).**\n`P2PSession::advance_frame`\nrecorded confirmed-input rows and their saved-state checksums before the\ncorrection `LoadGameState`/`SaveGameState` requests it had just queued\nwere\nexecuted by the application. Every frame recorded in a call that\nrepaired a\nmisprediction therefore captured its pre-correction speculative\nchecksum, so\n   decoding and playing that recording through a fresh session reported\n`ReplayDesync` at exactly the corrected frames even though the live\nhistory\n   was correct.\n2. **Close the #377 differential membership scope.** Model a second\ncertified\n   membership cut (a player host crash composed with the first explicit\nremoval), pin the removed player's non-withdrawn spectator endpoint\ncontract\n(certified history when a survivor is canonical, fail-closed when the\nremoved\n   endpoint is canonical), and add a long-run saved-state ring-wrap row.\n\n`PLAN.md`: the P0 differential rollback-equivalence milestone is\ncomplete; the\npartition/membership CAP matrix is promoted to the active P0 item.\n\n### Fix details\n\n- `pending_replay_checksum_refresh` tracks the lowest frame whose\nrecorded\nchecksum may be speculative; `adjust_gamestate` arms it when recording\nis\n  enabled.\n- `finalize_pending_replay_checksums` re-reads that range from the\nsaved-state\nring at the next `advance_frame` (after the application executed the\nprevious\ncall's requests) and before `take_replay` / `into_replay`. A frame whose\ncell\nis no longer buffered records no checksum rather than a value the\nsession has\n  since replaced.\n- `ReplayRecorder::set_checksum` provides the index-addressed overwrite.\n- Red evidence (pre-fix): 13 rollbacks recorded frames 5..11\nspeculatively and\nplayback reported `ReplayDesync` at frames 6,7,8,9,10,11. Green: the\nsame\nwitness replays cleanly with all 19 checksums present, and a positive\ncontrol\n  (one corrupted checksum) still reports a desync.\n\n### Test details\n\n- **Host crash composed with removal:** the reference carries two\nindependent\ncertified cuts; the bounded four-peer row (remove slot 1, then kill\nplayer\nhost 2 or 3 under `ContinueWithout`) proves survivor convergence past\nthe\nsecond cut, spectator failover with no divergence, cross-order\ncertified-prefix\nidentity, replay determinism, and a post-cut wrong-state mutation\ndetection.\n- **Removed player's spectator endpoint:** surviving canonical host\nkeeps the\n  exact certified frozen history; removed canonical host fails closed\n  (`SpectatorProgressMissing`) without forking.\n- **Ring wrap:** 480 steps at `max_prediction = 1` (two-cell ring), both\nsave\n  policies, all drive orders.\n\n### Dependency refresh\n\n`cargo update`: `itoa` 1.0.18 → 1.0.19, `syn` 3.0.9 → 3.0.10. Remaining\noutdated roots are deliberately pinned majors/minors.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `tests/sessions/replay_recording.rs` (new): records a two-peer session\nthat\n  repairs a burst of mispredictions and replays it through\n`ReplaySession::new_with_validation` for both `EveryFrame` and `Sparse`.\n- `tests/simulation/differential.rs`: `host_crash_after_removal_*`,\n\n`removed_player_spectator_endpoint_keeps_certified_history_or_fails_closed`,\n\n`saved_state_ring_wrap_with_minimal_prediction_window_matches_nominal_history`,\n  plus the composed-membership validation contract.\n\n**Manual testing performed:**\n\n- `cargo fmt` / `cargo clippy --workspace --all-targets --features\ntokio,json` /\n  `--all-features` → clean\n- `cargo nextest run --no-capture` → 3042/3042 passed, 73 skipped\n- `python3 scripts/ci/agent-preflight.py --auto-fix --files ...` → all\nchecks passed\n- `python3 scripts/hooks/check-plan-hygiene.py` → pass\n\n## Related Issues\n\nCloses #380\nCloses #377\n\n\n### Update: first #382 CAP-matrix row\n\n`tests/simulation/harness/components.rs` adds a declared component\nlayout\n(`RunOptions::component_scope`) and a component-scoped comparison\n(`classify_component_scope`) that distinguishes a failed-closed\npartition from a\none-sided split match and from two components exposing different values\nfor the\nsame confirmed frame (a fork). The first bounded row\n(`symmetric_partition_components_fail_closed_without_fork`) drives a\npermanent\n2/2 partition over four peers under `ContinueWithout`: both components\nstop at\nthe same cut, all peers end `Synchronizing`, no `Disconnected` event\nappears, the\nreviewed terminal failure set holds with no input/state divergence, and\na\nmutation control that rewrites both members of one component on the last\nshared\nframe flips the classification to `Forked`.\n\nMeasured note recorded on #382: a 3-peer singleton isolation (1/2 split)\nalso\nfails closed on every peer today, so no majority-continues component\nexists yet.\n\n\n### Update: CAP row 2 (timeout-versus-slow-peer) and #365 item 1\n\n- **CAP row 2:** a three-peer mesh where peer 2's application pauses for\n960 ms\n(below the 2 s timeout) must keep one confirmed history: classification\n`ContinuedTogether`, final confirmed 833/834/835, all peers `Running`,\nzero\n`Disconnected`/`PeerDropped`/`DesyncDetected`. The matched negative\ncontrol\npauses the same peer for 3.2 s: it is certified as dropped, the\nsurvivors move\n  on, and the branch is reported as `Forked` with an explicit\n`ConfirmedInputDivergence` — detected, never silent. `ComponentOutcome`\nnow\ndistinguishes `ContinuedTogether` from `SplitMatch`, and the component\ncut is\ncaptured at the first declared `PeerStall` as well as at a\ncross-component\n  `Block`.\n- **#365 ordered work item 1:**\n`.agents/skills/kani/references/kani-obligations.md`\n  records the exact `Frame::buffer_index` and varint contracts/domains/\n  assumptions/exclusions. Three new Tier-1 proofs pass locally:\n  `proof_frame_buffer_index_invalid_none` (2.3 s),\n  `proof_frame_buffer_index_valid_modulo` (2.4 s), and\n`proof_varint_full_u64_roundtrip_fixed_buffer` (6.3 s, every `u64`,\nten-byte\nbuffer). A symbolic 64-bit divisor exceeded 7 minutes (15-minute timeout\non the\nunbounded form), so arbitrary-size exactness is recorded as a Lean\ncandidate\n  instead of a claimed Kani result.\n\n\n### Update: CAP rows 3-4 and #365 ordered work item 2\n\n- **Row 3 (majority continuation):** a four-peer mesh with `PeerKill {\npeer: 3 }`\nkeeps survivors 0/1/2 `Running` and confirming to 573/573/574 while the\nkilled\n  peer stops at 114. Classification `SplitMatch { shared_prefix: 114,\ncontinued: [0] }`, no oracle failures, and a mutation control that flips\nit to\n  `Forked`.\n- **Row 4 (concurrent disjoint graceful removals):** two removals at the\nsame\nstep (0→1, 2→3) fail closed on every component at the shared cut with\nonly the\nreviewed drop-timeout/`EndProgress` diagnostics and zero\n`DesyncDetected` or\ninput/state divergence — an explicit terminal outcome, never a silent\nfork.\n- **#365 item 2 (extraction smoke test, pass):** `cargo-hax 0.4.2` +\nAeneas\n`build-183e4f0` + hax-lean `v0.3.30` + Lean 4.31.0 extract the unchanged\n`Frame::buffer_index` into Lean; `lake build` → `Build completed\nsuccessfully\n(1735 jobs)`. Recorded in the obligations reference with the exact pins\nand\n  the tuple-struct/derive `Unreachable` finding.\n\n\n### Update: #365 ordered work item 3 (production varint seam)\n\n- The private LEB128 implementation moved to the dependency-free\n`src/rle/varint.rs` with its own `VarintError`; `src/rle.rs` maps it\nonto the\nexact `FortressError` payloads. Differential tests compare decode\nagainst a\n  pre-refactor reference for every encoded prefix and offset, malformed\n  continuations, short-buffer prefix writes, and boundary roundtrips\n  (full suite 3056/3056).\n- The unchanged production file extracts through cargo-hax/Aeneas\n(pinned\n`build-183e4f0`, hax-lean `v0.3.30`) and the generated Lean package\nbuilds\n(1735 jobs). `encoded_len_zero` is proven; the loop-heavy contracts are\n  recorded as deferred with their full-domain Kani/Rust evidence.\n- Compiling-source mutation control: `encoded_len(0) → 2` still extracts\nbut\n  falsifies `encoded_len_zero`; the restored source verifies green.\n\n\n### Update: #365 item 4 (hardening/measurement/decision) and CAP row 5\n\n- **`lean-varint-pilot` harness:**\n`scripts/verification/lean-varint-pilot/`\ndeterministically re-extracts the unchanged production varint source,\nrejects\nstale/pin-drifted output through a normalized `MANIFEST.sha256`, builds\nthe\npackage, and audits axioms/proof holes. Measured: warm extraction ~1.1\ns,\nwarm verify 1.2 s ×3, cold project verify 118–136 s, dependency cache\n7.8 GB\n  raw / 2.8 GB gzip. Decision recorded: **not retained** — only\n`encoded_len_zero` is Lean-proven (Kani already covers the full\ncontract),\nloop-heavy proofs need `partial_fixpoint` induction, and no owner exists\nfor\n  the pinned stack.\n- **CAP row 5 (coordinator loss):** a four-peer mesh where the\ngraceful-removal\ncoordinator is killed before inventory commits fails closed on the\nsurvivors\nat the shared cut (113/114, `Synchronizing`), with zero `DesyncDetected`\nand\n  no input/state divergence; a mutation control flips the comparison to\n  `Forked`.\n\n\n### Update: heal/reflap probe found #383, and #365 is closed\n\n- A deterministic heal-after-partition probe found a post-heal split\n(peers 1\nand 3 resume on divergent histories at frames 96–103, no\n`DesyncDetected`).\nFiled as #383 with the exact schedule/evidence and tracked in `PLAN.md`.\nThe\ncomponent classifier now compares post-floor tails, which classifies the\nprobe\n  as `Forked { frame: 96, … }` instead of the previous `FailedClosed`.\n- #365 is closed with the rejected-experiment decision recorded: all\nfour\nordered items are complete, but the Lean lane does not yet prove a\nproperty\n  beyond Kani, the loop proofs are deferred, and no owner exists.",
          "timestamp": "2026-11-10T09:24:39-08:00",
          "tree_id": "d595075c0551670b68bd904c34c83e53322ab70a",
          "url": "https://github.com/wallstop/fortress-rollback/commit/582180484bc7a95c7c17c7c482c9c1721df45c61"
        },
        "date": 1794332075689,
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
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 81,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 276,
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
            "value": 61,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 156,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 525,
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
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 99,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 124,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 162,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 178,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 250,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 366,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 364,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 467,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 760,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 175,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 205,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 246,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 366,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 439,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 560,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 649,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 778,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1034,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 390,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 499,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 701,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 888,
            "range": "± 5",
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
            "value": 395,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 669,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 935,
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
            "value": 89,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 312,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1432,
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
          "id": "84161e7692628c1773630835f6856be604ca1d26",
          "message": "fix: survivor quorum for automatic drops + N-peer hot-join serve shape (close #383, #384) (#385)\n\n## Summary\n\nCloses the #383 red case and the #384 rejoin-during-membership-change\nrow. Leaves #382 open for the new asymmetric-partition finding (#386).\n\n### 1. Survivor quorum for automatic graceful drops (#383)\n\nA symmetric partition that outlives the disconnect timeout let each\ncomponent commit a complementary automatic drop certificate: every side\nhad an independent witness (its same-component partner), so both peers\nresumed on divergent confirmed histories after the link healed.\n\n- Automatic `ContinueWithout` timeouts now require both an independent\nwitness **and** a strict majority of the agreed membership epoch to\nremain locally reachable.\n- `UdpProtocol::has_reported_disconnect` exposes the latched\nsilence-timeout verdict so a timed-out (or recovered) endpoint cannot\ncount as a witness it never answered.\n- The quorum is re-checked when a queued automatic intent starts, not\nonly when it is enqueued.\n- Explicit `remove_player` keeps its application-directed one-survivor\npath.\n\nBefore (issue #383 schedule, 4 peers, cross-component block steps\n100-300):\n\n```\ntrace_hash=cdc59fcbac88641f\nfinal_confirmed=[94, 696, 94, 695]\nfinal_states=[Synchronizing, Running, Synchronizing, Running]\nForked { frame: 96, component: 0, other_component: 1 }\n```\n\nAfter:\n\n```\ntrace_hash=8ada5925d2449f99\nfinal_confirmed=[94, 94, 94, 94]\nfinal_states=[Synchronizing, Synchronizing, Synchronizing, Synchronizing]\nFailedClosed { shared_prefix: 94 }\nevents: zero Disconnected/PeerDropped, NetworkResumed observed\n```\n\nMajority shapes: N=3 2/1 → `final_confirmed=[774, 774, 94]`, `SplitMatch\n{ shared_prefix: 94, continued: [0] }`; N=8 6/2 → `[498 × 6, 94, 94]`,\nsame split match.\n\nMutation control: forcing the quorum helper to `true` fails the new\neven-split row, so it is load-bearing. Rewriting a component's value on\nthe shared frame flips both new rows to `Forked`.\n\n### 2. Hot-join rejoin during another removal certificate (#384, last\n#382 row)\n\nA 3+-machine joiner always builds as an N-peer joiner and requires\nbridge inputs. The host chose its serve arm by the currently reachable\nsurvivor set, so once the last other survivor left it fell back to the\n2-peer frame rewrite and every snapshot was rejected as \"the 2-peer\nserve shape\". The arm is now selected by the registered remote-machine\ncount; two-machine meshes keep the 2-peer flow.\n\nNew bounded differential row: three peers, `GracefulRemove { by: 0,\ntarget: 2 }` and `HotJoin { slot: 1 }` at step 100, `ContinueWithout`,\n`EveryFrame`. Both certificates commit independently (explicit removal\ncut 48 at commit step 108; hot-join removal 52, snapshot 95), the\ntrajectory is drive-order identical and exactly replayable, and a\nfrozen-slot lie plus a wrong post-activation state are both detected.\nMutation control: restoring the old `survivors.is_empty()` arm selection\nfails the row with the exact snapshot-shape violation.\n\nHarness modeling needed for the composition: the old generation keeps\npolling until its own removal commits (it may witness another\ncertificate); the offline replacement no longer consumes the live slot's\ninbox; the pending hot-join commit bounds only the sampled history by\nthe freeze cut; and the freeze-frame oracle compares only the freeze\nvalue for a replacement generation.\n\n## Verification\n\n- `cargo fmt --check`: clean.\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`: clean.\n- `cargo clippy --workspace --all-targets --features tokio,json,hot-join\n-- -D warnings`: clean.\n- Full `cargo nextest run` (default features): 3061 tests, zero failures\n(73 intentional skips).\n- Full `cargo nextest run --features hot-join`: 3324 tests, zero\nfailures (74 intentional skips).\n- `check-changelog-unreleased.py`, `check-plan-hygiene.py`,\n`sync-version.sh --check`: pass.\n- `markdownlint` and `typos` on every changed documentation file: clean.\n\n## Notes and follow-up\n\n- No wire-format or public API changes.\n- Docs/changelog cover both pre-existing behavior fixes.\n- A one-way partition sweep found a new red case: the isolated peer can\nstill receive, so it confirms ~8 frames past the majority's certified\ndrop before failing closed, exposing a divergent prefix. Filed as #386\nwith the exact schedule, measured evidence, and suggested directions;\nPLAN.md now records it as the remaining #382 row.\n\nCloses #383, #384. Refs #382, #386.",
          "timestamp": "2026-11-10T15:59:55-08:00",
          "tree_id": "1d9b67a711f0e984e7b6ed5d0907dbcdb7336217",
          "url": "https://github.com/wallstop/fortress-rollback/commit/84161e7692628c1773630835f6856be604ca1d26"
        },
        "date": 1794355650831,
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/8",
            "value": 19,
            "range": "± 2",
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
            "value": 86,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 216,
            "range": "± 0",
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
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 41,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 116,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 337,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 17,
            "range": "± 1",
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
            "value": 16,
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
            "value": 18,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 57,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 69,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 93,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 122,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 134,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 217,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 205,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 255,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 453,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 138,
            "range": "± 10",
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
            "value": 141,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 197,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 304,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 307,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 347,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 534,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 551,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 273,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 353,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 413,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 581,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 68,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 85,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 389,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 655,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 999,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 8,
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
            "value": 283,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1060,
            "range": "± 18",
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
          "id": "9c3b8584a414cf2ba22ac75ba75b019ba42220e7",
          "message": "test(simulation): codify one-way partition as a bounded terminal split (#386) (#387)\n\n## Description\n\nCloses the last open row of the P0 CAP matrix (#382): a one-way\npartition lets the isolated peer keep confirming its prediction window\nafter the majority's certified cut because it still receives the\nmajority's inputs while none of its own arrive. #386 recorded the\nresulting divergence between the isolated peer's live slot values and\nthe majority's frozen prefix.\n\n**Ack-bounding `confirmed_frame` was prototyped and measured, then\nrejected for this change.** Folding each endpoint's `last_acked_input`\ninto the local slot's confirmed contribution makes #386 clean, but it\nlowers confirmation by a round trip on every lossy link and shrinks the\neffective rollback window. Measured blast radius:\n`test_determinism_under_stress` dropped to **38/100** simulated frames\nunder `ChaosConfig::terrible_network` (250 ms latency, 15 % loss),\n`test_eventual_consistency` to 56/60, every baseline drifted,\ndifferential survivor liveness failed across rows, and long\nGilbert–Elliott one-way loss made acknowledged frames jump farther than\nthe bounded input ring can hold (`confirmed_inputs_for_frame`\nunavailable). It remains the clean eventual fix only alongside a\nseparate pacing/queue redesign; this session records the contract and\nevidence.\n\nThis change makes the limitation explicit instead:\n\n- `ComponentOutcome::SplitTail` reports a failed-closed component's\nbounded post-cut tail. `classify_component_scope_with_tail` accepts it\nonly when the first divergence is strictly after the delivery-ambiguity\nwindow, one side is a Running survivor and the other is\nall-`Synchronizing`, every survivor pair agrees over its full shared\nrange, and every failed-closed component's floor is within the caller's\ntail allowance. All other shapes remain `Forked`.\n- `docs/reconnect-playbook.md` (and its wiki mirror) states the\ncontract: the majority's certified history is authoritative, the\nisolated peer's post-cut tail is discarded, the prediction window bounds\nit (plus one frame of cut-observation slack), and recovery requires\nrebuilding the isolated session.\n- `PLAN.md` drops the completed P0 CAP-matrix item; the next queued work\nis the P1 list.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy** (no production code\nchanged)\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n(none: no behavior changed; the existing behavior is now documented as\nan explicit contract)\n- [ ] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n-\n`asymmetric_one_way_partition_isolated_peer_exposes_only_a_bounded_tail`:\n3 peers, `ContinueWithout`. Reproduces #386 (pre-contract `[774, 774,\n104]`, divergence 96–104), now classifies as `SplitTail { shared_prefix:\n95, continued: [0], tail: 9 }` with a `max_prediction + 1` allowance;\nevery oracle divergence is strictly after the shared prefix; exact\nreplay; mutation controls flip a shared-prefix rewrite and an overlong\ntail to `Forked`.\n- `asymmetric_one_way_partition_variants_expose_only_bounded_tails`:\n3-peer `Halt`, 4-peer `ContinueWithout`, 4-peer `Halt`, each a legal\nbounded `SplitTail` with the expected survivor set and exact replay.\n- Four classifier unit tests: bounded split tail, overlong tail,\ndiverging-but-Running terminal, and divergence inside the cut window.\n\n**Verification:**\n\n- `cargo nextest run --no-fail-fast`: **3067/3067 passed** (73 skipped).\n- `cargo nextest run --features hot-join --no-fail-fast`: **3330/3330\npassed** (74 skipped).\n- `cargo fmt --check` and `cargo clippy --workspace --all-targets\n--features tokio,json -- -D warnings`: clean.\n- `python3 scripts/ci/agent-preflight.py --auto-fix`: all checks passed.\n- `cargo update`: no compatible updates (six root dependencies remain\ndeliberately pinned majors/minors).\n\n## Related Issues\n\nCloses #386\nCloses #382\nCloses #384",
          "timestamp": "2026-11-10T18:08:46-08:00",
          "tree_id": "ccc526c74d5e776874d5423ff83b8650cacb08d5",
          "url": "https://github.com/wallstop/fortress-rollback/commit/9c3b8584a414cf2ba22ac75ba75b019ba42220e7"
        },
        "date": 1794363433087,
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
            "value": 23,
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
            "value": 86,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 341,
            "range": "± 2",
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
            "value": 35,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 48,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 129,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 488,
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
            "value": 84,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 105,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 137,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 149,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 219,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 302,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 280,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 379,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 551,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 171,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 167,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 198,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 298,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 319,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 420,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 564,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 639,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 803,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 336,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 434,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 583,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 735,
            "range": "± 4",
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
            "value": 514,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 866,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1269,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 12,
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
            "range": "± 2",
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
            "value": 1173,
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
          "id": "cb63e96879cfd012114950612fe32dcfdf3664e5",
          "message": "test(simulation): qualify pacing/liveness cliffs with ablations (#389)\n\n## Summary\n\nQualifies the next PLAN P1 — measured pacing/liveness cliffs — with a\ndeterministic ablation rather\nthan a speculative controller change. `tests/simulation/pacing_cliff.rs`\nreproduces the checked-in\nmobile 2-player and severe 4-player gate rows (seed 1, 1,000 steps, 16\nms virtual steps) and varies\n`max_prediction`, `input_delay`, and application wait compliance.\n\n### Measured findings\n\n| Cell | Max prediction | Input delay | Advance stalls / 16 s |\nResimulated frames | Mean confirmation lag |\n| --- | ---: | ---: | ---: | ---: | ---: |\n| 2 / mobile | 8 | 1 | 8 | 5,011 | 4.33 |\n| 2 / mobile | 16 | 1 | 0 | 4,993 | 4.36 |\n| 4 / severe | 8 | 1 | 2,202 | 3,815 | 6.69 |\n| 4 / severe | 16 | 1 | 664 | 18,028 | 14.12 |\n| 4 / severe | 24 | 1 | 102 | 24,776 | 16.98 |\n| 4 / lossless 200 ms RTT | 8 | 1 | 1,948 | 5,318 | 6.90 |\n| 4 / 10 ms RTT, 15% loss | 8 | 1 | 138 | 4,448 | 4.54 |\n| 4 / 10 ms RTT, lossless | 8 | 1 | 0 | 3,721 | 3.77 |\n\n- **Latency headroom dominates loss recovery.** A *lossless* 200 ms path\nstalls nearly as much as\nthe 15% loss severe row (1,948 vs 2,202), while 15% loss at 10 ms RTT\ncosts only 138 stalls.\nRetry cadence and loss cannot explain the severe cliff; the fixed\nprediction window versus the\n  slowest one-way path can.\n- **Mobile's residual stalls are free to remove.** 8→12 cuts stalls 8→2\nwithout additional\nresimulation; 16 removes them. The mobile row is already at its\nresimulation plateau.\n- **Severe's window is a capacity boundary.** 8→16 cuts stalls 70% but\ncosts 4.7× resimulation,\nand 24 frames still records 102 stalls. Raising the default blindly is\nnot retained.\n- **`WaitRecommendation` cannot substitute for window sizing.** Under\nuniform high RTT the\nsymmetric `RTT/2` frame-age estimate cancels most of the transport\noffset from the\nframe-advantage signal. The severe row emits zero recommendations and\nrecords the identical\n2,202 stalls with `AppModel::Obey`; mobile emits two recommendations,\nobeys three frames, and\n  moves stalls only 8→7.\n- **Input delay is a tuning lever, not a cure.** At a fixed eight-frame\nwindow, delay 3 cuts\nsevere stalls 20% and resimulation 31% while adding 33 ms of local input\nlatency.\n\nThis matches the PLAN item's exit condition for retaining no\ncontroller/configuration change: the\nmechanism is transport headroom, not oscillation or an avoidable pacing\nbug. The clock-skew\nsawtooth and H-BLOAT drain behavior already have their own deterministic\nprobes and are unchanged.\n\n### Changes\n\n- `tests/simulation/pacing_cliff.rs`: five data-driven tests covering\nthe prediction-window sweep,\nlatency-vs-loss isolation, wait compliance, input-delay tradeoff, and\nbaseline calibration.\n- `sweep_pr_gate` now gates `stalls_per_min` (25% + 2/min) and\n`confirmation_lag_mean`\n(10% + 0.25 frames) one-sided against the checked-in ledger, so the\ncliffs cannot move silently;\n  genuine improvements still pass.\n- `docs/tuning.md` (+ generated wiki mirror) records the ablation table,\nthe silent-advisory\n  mechanism, and the tradeoffs.\n- `PLAN.md`: completed P1 item removed and the remaining queue\nrenumbered.\n\n## Testing\n\n- `cargo fmt --all --check`\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`\n- `cargo nextest run --features tokio,json --no-capture` — full suite\ngreen\n- `python3 scripts/ci/agent-preflight.py --auto-fix` — all checks passed\n- `markdownlint`, `typos`, `vale docs/tuning.md`,\n`check-plan-hygiene.py`, `check-doc-claims.sh`\n- New `pacing_*` and `latency_headroom_*` tests plus `sweep_pr_gate`\nreproduce the table above\n\nRefs #388. The acknowledgement-bounded follow-up would fold an extra\none-way trip into\n`confirmed_frame`; this data quantifies why that directly consumes the\nheadroom shown here, but\nthis PR changes no protocol behavior.",
          "timestamp": "2026-11-10T19:58:40-08:00",
          "tree_id": "df795dcfb97ab00a9f28fc1eda959b4c4351d2b7",
          "url": "https://github.com/wallstop/fortress-rollback/commit/cb63e96879cfd012114950612fe32dcfdf3664e5"
        },
        "date": 1794370065061,
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
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 87,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 298,
            "range": "± 9",
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
            "value": 40,
            "range": "± 2",
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
            "value": 158,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 579,
            "range": "± 9",
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
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/16",
            "value": 27,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/64",
            "value": 28,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 42,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 90,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 115,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 157,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 155,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 214,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 328,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 287,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 404,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 668,
            "range": "± 37",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 152,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 172,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 221,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 299,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 360,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 480,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 537,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 651,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 914,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 360,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 474,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 672,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 865,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 100,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 139,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 421,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 682,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 972,
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
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 92,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 349,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1381,
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
          "id": "a40e0edae66924b127c77c7ceeca519420bb9dfb",
          "message": "feat(transport): expose adapter-level socket metrics (#391)\n\n## Summary\n\nStarts the next queued PLAN item by making adapter-level transport work\nobservable. Previously\n`PeerMetrics` reported protocol enqueue demand only: callers could not\ndistinguish accepted sends\nfrom adapter drops, raw ingress from decoded messages, or a receive-poll\ncap saturation from an idle\npoll.\n\n- New public `SocketMetrics` with `receive_polls`, `datagrams_received`,\n`bytes_received`,\n`messages_decoded`, `datagrams_rejected`, `receive_cap_saturations`,\n`datagrams_sent`,\n  `bytes_sent`, and `send_failures`.\n- `SessionMetrics::socket` carries the live adapter snapshot for\n`P2PSession` and\n`SpectatorSession`; replay/sync-test sessions keep the default zero\nvalue.\n- `NonBlockingSocket::socket_metrics` is a provided trait method\nreturning zeroed counters, so\nexisting custom adapters compile unchanged. Built-in UDP and Tokio\nadapters override it; the\n  chaos wrapper delegates to its inner adapter.\n- `receive_all_messages_from` records raw datagrams/bytes, decoded\nmessages, decode rejections, and\nraw/decoded per-poll cap saturation without changing the existing\n256-attempt / 256-message\n  bounds.\n- Docs, changelog, public API snapshot, and PLAN in-progress item\nupdated.\n\n### Why\n\nThis closes the \"record actual socket throughput/loss, not only enqueued\nbytes\" and \"overload is\nvisibly refused or degraded\" parts of the transport P1 evidence: rising\n`socket.send_failures`\nidentifies adapter drops, and rising `socket.receive_cap_saturations`\nidentifies sustained ingress\nsaturation. A successful UDP send remains best-effort and is documented\nas adapter acceptance, not\nproof of delivery.\n\n## Tests\n\n- New real-loopback UDP test: accepted send/bytes counters, decoded\nreceive counters, then a raw\nmalformed datagram increments `datagrams_rejected` without changing\ndecoded counts or cap\n  saturation.\n- Expanded deterministic receive-cap test: 256 malformed datagrams\nreport 256 raw receives, 256\nrejects, 0 decoded, and exactly one cap saturation; the valid datagram\nbehind them decodes on the\n  next poll.\n- New session test pins the adapter→`SessionMetrics::socket` merge path.\n- `cargo fmt --all --check`\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`\n- `cargo nextest run --features tokio,json`: 3,109 passed, 73 skipped, 0\nfailed\n- `python3 scripts/ci/agent-preflight.py --auto-fix`: all checks passed\n- `python3 scripts/api/public_api_census.py --check`: 2,742 symbols\nmatch\n- `markdownlint`, `typos`, changelog/version/plan hooks: clean\n- Diff mutation gate: adapter `socket_metrics` overrides and\n`record_send_failure` are caught; the two\nsemantically-equivalent default-trait delegates and mutually-exclusive\nchaos wrappers are excluded\n  in `.cargo/mutants.toml` with rationale\n\nTransport P1 remains in progress: retransmission storms, browser\nsuspension, a real-transport\nthroughput/loss sampler, and downstream queue-age evidence still remain.",
          "timestamp": "2026-11-10T22:32:49-08:00",
          "tree_id": "6ef79a19821fa77167b4def144ffd3274b949345",
          "url": "https://github.com/wallstop/fortress-rollback/commit/a40e0edae66924b127c77c7ceeca519420bb9dfb"
        },
        "date": 1794379272455,
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
            "value": 26,
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
            "value": 232,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 31,
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
            "value": 122,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 449,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 70,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 87,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 122,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 120,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 165,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 254,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 224,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 312,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 499,
            "range": "± 4",
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
            "value": 133,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 184,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 232,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 279,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 365,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 417,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 516,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 718,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 275,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 365,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 521,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 667,
            "range": "± 18",
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
            "value": 110,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 339,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 543,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 775,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 30,
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 72,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 275,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1080,
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
          "id": "de1cdf9d09c9e16e45eaf95c2170f0ca26014c41",
          "message": "test(transport): complete real-path bounds and scope ack exposure (#392)\n\n## Summary\n\nFinishes the active P1 transport slice and scopes the #388 follow-up. No\nprotocol or wire behavior changes.\n\n### Transport completion\n\n- `SocketMetrics` gains `queue_depth` and `oldest_queued_ms`. Built-in\nUDP/Tokio report zero (direct\n  submit); `ChaosSocket` reports its held-packet queue.\n- New real-loopback sampler: 100 datagrams through a seeded 25%\n`ChaosSocket` send-loss path.\nAccepted sends + adapter drops equal 100, delivered traffic is a\npositive subset of accepted, and\nthe loop's decoded count matches the receiver adapter counter. This\nrecords actual adapter\n  acceptance/loss instead of inferring from enqueue demand.\n- New deterministic 150-step (2.4 s) peer-suspension row under an\nexplicit 5 s disconnect timeout.\nBoth peers resume, confirm >500 frames, stay within the prediction\nwindow, and replay exactly.\n- New queue-age test: a 200 ms shaping queue reports depth 1/age 0,\ndepth 1/age 75 ms after 75 ms of\n  virtual time, and depth 0/age 0 after delivery.\n- Existing evidence retained and cited: 256 raw/decoded per-poll cap\nwith next-poll valid recovery;\n128-frame pending-output retransmission boundary and fail-closed beyond;\nbandwidth/MTU/rebind\n  census rows; protocol-valid lie coverage.\n- P1 transport item removed from `PLAN.md`; docs now include a\n\"Transport bound evidence\" section.\n\n### #388 scope\n\nRecorded an explicitly scoped, falsifiable experiment in\n`.agents/skills/design-decisions/references/network.txt` and posted it\non #388:\n\n1. Receipt-only confirmed frame (current bounded split tail).\n2. Always fold `last_acked_input.frame` into `confirmed_frame()`\n(rejected prototype).\n3. Gate only when the unacked span exceeds half the prediction window.\n\nInstrument `last_acked_input.frame`, `confirmed_frame()`, and survivor\nprogress on the #386 one-way\nrow plus `ChaosConfig::terrible_network`; accept only if the cut tail is\n≤1 frame and survivor\nprogress stays within 10% of baseline. The #389 baseline is 6.90/8\nframes mean lag on a lossless\n200 ms 4-peer row, so always-ack-bounding cannot fit the default window.\n\n## Verification\n\n- `cargo nextest run --features tokio,json`: 3,112 passed, 73 skipped, 0\nfailed\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`: clean\n- `agent-preflight.py --auto-fix`, markdownlint, typos,\nchangelog/version/plan hooks: clean\n- Public API census: 2,746 symbols, snapshot matches\n- Diff mutation: the new `ChaosSocket::socket_metrics_impl` mutant is\ncaught by the queue-age test",
          "timestamp": "2026-11-11T00:09:30-08:00",
          "tree_id": "658d76b6b365e28acfd1718e05b63af25bc9dab5",
          "url": "https://github.com/wallstop/fortress-rollback/commit/de1cdf9d09c9e16e45eaf95c2170f0ca26014c41"
        },
        "date": 1794384991711,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 21,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 50,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 203,
            "range": "± 8",
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
            "value": 25,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 70,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 269,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/4",
            "value": 15,
            "range": "± 1",
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
            "value": 21,
            "range": "± 1",
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 60,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 74,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 88,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 109,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 182,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 172,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 218,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 314,
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
            "value": 93,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 107,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 161,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 184,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 258,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 331,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 377,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 468,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 220,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 264,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 334,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 438,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 59,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 75,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 339,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 570,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 867,
            "range": "± 13",
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
            "value": 40,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 157,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 796,
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
          "id": "2a360dbd46c8bd4a1d6c2f6ae0d450693cc0c1df",
          "message": "test(sim): measure ack-bound exposure and reject both candidates (#393)\n\n## Summary\n\nImplements the scoped #388 experiment end-to-end. No protocol behavior\nchanges: the measured\nresult rejects both acknowledgement-bounded candidates and retains the\nbounded terminal-tail\ncontract.\n\n### Instrumentation\n\n- `UdpProtocol::last_acked_input_frame` exposes the newest acknowledged\nlocal input.\n- `P2PSession::diagnostic_ack_exposure` returns each remote endpoint's\nacknowledged local frame\n  through a doc-hidden diagnostic surface.\n- Simulation harness `RunOptions::ack_exposure_probe_at` captures\ncurrent/confirmed/last-acked per\n  peer at one end-of-step boundary (`RunReport::ack_exposure_probe`).\n- The real `ChaosConfig::terrible_network` arm samples the same gauges\nevery frame on the\n  deterministic virtual clock.\n\n### Measured result\n\n**#386 one-way row** (3 peers, `ContinueWithout`, 900 steps, probe at\ncut+50):\n\n```\nshared=95 tail=9 current=112 confirmed=104 last_acked=95 max_prediction=8\n```\n\n- Receipt-only (current): documented bounded 9-frame tail.\n- Always ack-bound and half-window gating: floor 95, tail 0 — both meet\nthe tail goal.\n\n**Terrible network** (2 peers, 250 ms latency, 15% loss, seeds 42/43):\n\n```\npeak always-bound span = [16, 17] frames, half_gate_active=true, max_prediction=8\n```\n\nBoth candidates would stall survivor progress because their reverse-trip\nfloor exceeds the entire\nprediction window. The rejected prototype's 38/100 frames is the\nexpected consequence.\n\n### Decision\n\nRetain the bounded terminal-tail contract in\n`docs/reconnect-playbook.md` and `docs/tuning.md`.\nRemoving the tail without the liveness cost requires a bounded input\narchive plus a pacing-aware\nexposure policy. The design-decision log records the rejection and this\nprerequisite.\n\n## Tests\n\n- `ack_exposure_candidates_reject_healthy_liveness`: #386 exact replay,\ntail/goal assertions,\nmutated-ack control proving the result depends on the measured ack lag.\n- `terrible_network_ack_exposure_rejects_always_bound`: deterministic\nterrible-network run, exact\n  replay, peak always-bound span and half-window gate assertions.\n- `cargo nextest run --features tokio,json`: 3,114 passed, 73 skipped, 0\nfailed\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`: clean\n- `agent-preflight.py --auto-fix`, markdownlint, typos, version/plan\nhooks: clean\n- Public API census: 2,749 symbols, snapshot matches\n- Targeted mutation: `diagnostic_ack_exposure` and\n`last_acked_input_frame` mutants caught",
          "timestamp": "2026-11-11T01:51:23-08:00",
          "tree_id": "870137a3f2c0041e07660b3c44bf7cf9df3683f5",
          "url": "https://github.com/wallstop/fortress-rollback/commit/2a360dbd46c8bd4a1d6c2f6ae0d450693cc0c1df"
        },
        "date": 1794391170437,
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
            "value": 26,
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
            "value": 232,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 25,
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
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 122,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 448,
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
            "value": 70,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 87,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 120,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 120,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 162,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 254,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 223,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 307,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 483,
            "range": "± 13",
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
            "value": 133,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 171,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 220,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 263,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 348,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 417,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 501,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 693,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 271,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 373,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 525,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 698,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 81,
            "range": "± 2",
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
            "value": 365,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 564,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 792,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 30,
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
            "value": 266,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1063,
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
          "id": "d6be05cc2ab494c9a80d5f8c6474672a6d884209",
          "message": "test(perf): qualify representative-state tiers and publish budgets (#394)\n\n## Summary\n\nStarts the representative-state gameplay/performance qualification with\na real release workload\nledger. No production behavior changes.\n\n### Harness\n\n`tests/representative_perf.rs` runs real `P2PSession` meshes over an\nin-memory fabric with\nconfigurable snapshot payload, per-frame simulation bytes, player count,\ninput delay, prediction\nwindow, and deterministic message loss. It records:\n\n- frame-service p50/p95/p99/p99.9\n- rollback count, resimulated frames, max depth\n- allocation count and net allocated bytes\n- per-player protocol sent/received bytes\n- minimum final confirmed frame\n\nA 48-frame 2-player smoke row runs in the normal suite. The full\n2/4/16-player ledger is a\nrelease-only ignored test that emits JSONL via `FORTRESS_PERF_OUT` and\nenforces reference p99\nbudgets only with `FORTRESS_PERF_ENFORCE=1`.\n\n### Reference measurements\n\n| Tier | Players | State | Work | p50 | p99 | p99.9 | Rollbacks | Max\ndepth |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n| 2p-rep-64k | 2 | 64 KiB | 1 MiB | 0.035 ms | 0.076 ms | 0.105 ms | 294\n| 2 |\n| 4p-rep-256k | 4 | 256 KiB | 4 MiB | 0.181 ms | 0.499 ms | 0.625 ms |\n729 | 6 |\n| 16p-rep-64k | 16 | 64 KiB | 2 MiB | 0.084 ms | 0.141 ms | 0.149 ms |\n1,839 | 3 |\n\nReference budgets: p99 ≤ 2 ms for 2/4 players and ≤ 4 ms for 16 players\nat the recorded workloads.\nThe row uses instant transport and zero configured input delay to\nisolate snapshot/simulation cost;\nthe existing baseline sweep remains the source for\nmobile/4-player/16-player network-latency cost.\n\n`docs/tuning.md` publishes the command, table, thresholds, and\ninterpretation. The P1 item stays in\nprogress for the playable workload, visible correction/audio\nduplication, queue behavior, and\ntarget-hardware profiling.\n\n## Testing\n\n- `cargo nextest run --features tokio,json`: 3,115 passed, 74 skipped, 0\nfailed\n- Release ledger with `FORTRESS_PERF_ENFORCE=1`: all 3 tiers pass\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`: clean\n- `agent-preflight.py --auto-fix`, markdownlint, typos, version/plan\nhooks: clean\n- Dependencies: `cargo update --workspace --dry-run` selects zero\npackages; only MSRV-held majors\n  remain",
          "timestamp": "2026-11-11T03:31:20-08:00",
          "tree_id": "0e917a07090e4ced70eda0415826fda529285b1a",
          "url": "https://github.com/wallstop/fortress-rollback/commit/d6be05cc2ab494c9a80d5f8c6474672a6d884209"
        },
        "date": 1794397185556,
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
            "value": 35,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 81,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 276,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 40,
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
            "value": 152,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 520,
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
            "value": 99,
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
            "value": 162,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 179,
            "range": "± 3",
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
            "value": 368,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 342,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 468,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 717,
            "range": "± 19",
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
            "value": 207,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 245,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 352,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 415,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 529,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 648,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 772,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1032,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 376,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 495,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 694,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 905,
            "range": "± 5",
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 388,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 653,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 908,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 36,
            "range": "± 8",
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
            "range": "± 1",
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
            "value": 1427,
            "range": "± 30",
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
        "date": 1794401940032,
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
            "value": 26,
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
            "value": 231,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 31,
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
            "value": 123,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 448,
            "range": "± 7",
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
            "value": 69,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 87,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 120,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 119,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 166,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 254,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 223,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 312,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 498,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 117,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 133,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 171,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 220,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 262,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 356,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 417,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 510,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 705,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 281,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 368,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 530,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 670,
            "range": "± 11",
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
            "value": 110,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 332,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 552,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 757,
            "range": "± 21",
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
            "value": 267,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1065,
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
          "id": "ffe6c52d5ffd0208b18c41d77ac6ebedb4f7b1ea",
          "message": "fix(examples): propagate request errors instead of warning (#397)\n\n## Summary\n\nCompletes the P1 safe-integration item. Canonical request loops now fail\nearly on missing rollback\nstate and checksum serialization errors instead of warning and\ncontinuing.\n\n### Changes\n\n- New public `try_handle_requests!`: each save/load/advance closure\nreturns a `Result`, and the\nfirst error aborts the request loop. `handle_requests!` remains the\ninfallible convenience form.\n- `examples/request_handling.rs` returns `Result<(),\nRequestHandlingError>` from both manual and\nmacro paths; missing `LoadGameState` state and `ChecksumError`\npropagate.\n- `examples/sync_test.rs` returns a structured internal error for a\nmissing rollback save.\n- `docs/getting-started.md` and `docs/user-guide.md` canonical loops\npropagate errors; macro\n  guidance uses `try_handle_requests!`.\n- New `tests/sessions/misuse.rs` counterexamples: missing saved state,\nchecksum serialization\n  failure, omitted local input (`MissingLocalInput`).\n- New macro regression test proves later requests are not processed\nafter an error.\n- Production checklist maps event starvation, variable-width input,\nconfig mismatch, and NAT\n  rebinding to their deterministic suites.\n- Changelog Added entry; public API census refreshed (2,750 symbols).\n- P1 safe-integration item removed from `PLAN.md`; two future items\nremain.\n\n## Testing\n\n- `cargo nextest run --features tokio,json`: 3,119 passed, 74 skipped, 0\nfailed\n- `cargo clippy --workspace --all-targets --features tokio,json,hot-join\n-- -D warnings`: clean\n- `cargo run --example request_handling`: completes with checksums and\nno warn-and-continue path\n- `agent-preflight.py --auto-fix`, markdownlint, typos,\nchangelog/version/plan hooks: clean\n- `cargo update --workspace --dry-run`: 0 compatible updates",
          "timestamp": "2026-11-11T06:17:46-08:00",
          "tree_id": "47be49114991b630a2c5db0bbf782e19864d3ba8",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ffe6c52d5ffd0208b18c41d77ac6ebedb4f7b1ea"
        },
        "date": 1794407124355,
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
            "value": 26,
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
            "value": 232,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 31,
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
            "value": 122,
            "range": "± 1",
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
            "value": 70,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 89,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 120,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 120,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 166,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 253,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 223,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 313,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 483,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 118,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 133,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 172,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 220,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 260,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 357,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 417,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 509,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 710,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 269,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 366,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 538,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 683,
            "range": "± 2",
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
            "value": 110,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 333,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 539,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 766,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 30,
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
            "value": 67,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 266,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1064,
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
          "id": "8294aeadcbfa61356c84a716499de6470f778b66",
          "message": "test(replay): cross-platform replay, mixed transports, and tab suspension (P1) (#400)\n\n## Description\n\nAdvances PLAN P1 (portability and long-session determinism) with three\nevidence classes:\n\n- **Mixed transport replay.** Adds\n`tests/simulation/corpus/002-mixed-transport-quorum.json`, a\nchecked-in 4-peer, 1000-step schedule whose 12 directed links split\nevenly between a reliable\nretransmission transport (every would-drop is converted to a 40ms\nhead-of-line retransmit delay)\nand unreliable UDP-like loss (2% drop, 1% duplication, 25ms base + 10ms\njitter). A hold/release\nand a temporary block cross both classes before the final heal. The\ncorpus replay and strict\nsafe-release lanes run the full oracle and a second replay asserts an\nidentical trace hash and\n  final confirmed frames.\n- **One captured replay on every target.** Adds the\n`fortress-replay-fixture` test-support crate\nwith a 6171-byte replay captured from a real two-peer session with\nrollback corrections. Native,\nbrowser `wasm32-unknown-unknown`, and Godot `wasm32-unknown-emscripten`\nlanes load the same bytes,\nreplay with checksum validation, and assert frames=149, requests=298,\nzero `ReplayDesync`\nevents, and request/state digest `0xb3535a3db29da7784abaacf609fec42b`. A\ncorrupted-checksum\n  negative control proves the validation path has teeth.\n- **Background-tab suspension.** The browser runtime test now injects a\nvirtual clock. A 1s\nsuspension (above the 500ms interruption notice, below the 2s timeout)\nmust emit\n`NetworkInterrupted` then `NetworkResumed`, must not disconnect, and\nmust keep peer states\nidentical after resuming. A suspension past 2s must emit `Disconnected`\nfail-closed.\n\nPLAN now records extended frame/ring operation as the remaining P1 item.\nThe production checklist\nand its wiki mirror document the new cross-platform and mixed-transport\nevidence.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix\n- [ ] ✨ New feature\n- [ ] 💥 Breaking change\n- [x] 📚 Documentation\n- [ ] ♻️ Refactor\n- [x] 🧪 Test\n- [x] 🔧 CI/Build\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] Zero-panic policy: no new production code, and the test-support\ncrate returns `Result` for all\n      fallible replay operations.\n- [x] Added tests that prove the evidence is real (mixed-class counters,\nfull oracle, replay digest,\ncorrupted-checksum negative control, suspension resume and timeout).\n- [x] `cargo fmt` and `cargo clippy --workspace --all-targets --features\ntokio,json -- -D warnings`\n      pass.\n- [x] `cargo nextest run --features tokio,json` passes.\n\n### If Applicable\n\n- [x] Documentation updated (production checklist + wiki mirror + PLAN).\n- [ ] `CHANGELOG.md`: no public API or user-observable behavior changed;\ntest/CI/docs only.\n- [ ] Examples: not affected.\n- [x] No new compiler warnings.\n\n## Testing\n\n**Tests added/modified:**\n\n-\n`simulation::corpus_replay::checked_in_mixed_transport_schedule_replays_with_both_transport_classes`\n-\n`fortress-replay-fixture::tests::checked_in_fixture_replays_with_expected_evidence`\n-\n`fortress-replay-fixture::tests::mutated_recorded_checksum_is_detected`\n-\n`wasm-browser-smoke::captured_native_replay_replays_identically_in_browser`\n-\n`wasm-browser-smoke::browser_tab_suspension_within_timeout_resumes_and_advances`\n-\n`wasm-browser-smoke::browser_tab_suspension_past_timeout_disconnects_fail_closed`\n- Godot Emscripten probe + Playwright assertions for the same replay\nevidence.\n\n**Local evidence:**\n\n```\ncargo nextest run --features tokio,json\n  Summary [167.933s] 3120 tests run: 3120 passed, 74 skipped\n\ncargo nextest run -p fortress-replay-fixture\n  Summary 2 tests run: 2 passed\n\ncargo nextest run --test simulation -E 'test(simulation::corpus_replay)'\n  Summary 10 tests run: 10 passed\n  (mixed schedule: retransmit_delayed=6940, dropped_by_policy=137,\n   final confirmed=[813, 813, 813, 813])\n\ncargo clippy --workspace --all-targets --features tokio,json -- -D warnings\ncargo clippy -p wasm-browser-smoke --target wasm32-unknown-unknown --all-targets -- -D warnings\ncargo clippy --manifest-path tests/godot-emscripten/Cargo.toml --locked --all-targets --all-features -- -D warnings\ncargo shear, actionlint, agent-preflight, markdownlint, typos, doc links, doc claims: clean\n```\n\nBrowser and Emscripten execution evidence is produced by the hosted\n`WASM Build Check` and\n`Godot Emscripten Browser` jobs on this PR; the local container has no\nChrome or emsdk. The\nsuspension/resume/timeout semantics were pre-validated natively through\nthe same virtual-clock\nsequence (interrupted+resumed under 1s, interrupted+disconnected past\n2s).\n\n## Related Issues\n\nRelates to PLAN P1 (portability and long-session determinism). No open\nissues at session start.",
          "timestamp": "2026-11-11T19:24:48-08:00",
          "tree_id": "6132f26d0516163200c55089d42821d9ae90cdf0",
          "url": "https://github.com/wallstop/fortress-rollback/commit/8294aeadcbfa61356c84a716499de6470f778b66"
        },
        "date": 1794454351773,
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
            "value": 26,
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
            "value": 231,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 25,
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
            "value": 45,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 122,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 448,
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
            "value": 70,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 89,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 121,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 120,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 162,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 254,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 236,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 323,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 516,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 130,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 149,
            "range": "± 1",
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
            "value": 233,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 263,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 365,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 417,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 510,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 693,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 269,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 374,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 538,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 669,
            "range": "± 9",
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
            "value": 112,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 329,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 553,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 752,
            "range": "± 11",
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
            "value": 19,
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
            "value": 267,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1065,
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
          "id": "0b76a47782d8a44e88ccab298536c60e85519438",
          "message": "test(replay): extended frame/ring evidence and nightly host-stall tolerance (#401)\n\n## Description\n\nCompletes the final PLAN P1 item — extended frame/ring operation — and\nfixes the recurring macOS\nnightly real-UDP host-stall failure.\n\n- **Extended portable replay.** The checked-in two-peer capture grows\nfrom 149 to 4,096 replayed\nframes (68.3 s at 60 Hz). It recycles the 128-frame input queue 32 times\nand the default\nnine-cell saved-state ring more than 450 times. Playback now records an\nordered request-stream\ndigest and re-encodes the decoded replay to assert the checked-in\nartifact is canonical, so\nnative, browser `wasm32-unknown-unknown`, and Godot\n`wasm32-unknown-emscripten` lanes prove both\nthe request sequence and the serialized artifact are byte-identical.\nNon-vacuity tests assert\nthe ring-cycle counts and that every replayed frame carries a recorded\nchecksum; the existing\n  corrupted-checksum control still detects a mutated checksum.\n- **Extended live corpus schedule.** New\n`tests/simulation/corpus/003-extended-ring-recycling.json` runs 4 peers\nfor 16,384 steps at the\nminimum prediction window over mixed reliable-retransmission and\nunreliable links. Sixteen\nhold/block/stall/delay-increase cycles keep rollback and ring recycling\nactive, and the\ndedicated replay test asserts the full oracle, final confirmed prefixes\npast 12,000 frames, and\n  identical trace identity on a second run.\n- **Nightly host-stall RCA (closes #390).** The scheduled nightly run on\n`4a84892` failed only\nmacOS `test_extended_chaos_bursty_survivable` after 180 s with both\npeers in `Synchronizing`\n(`current=8 confirmed=NULL_FRAME`, `synced=1 interrupted=1\ndisconnected=1`) — the same signature\nas #390. A hosted runner can lose the CPU for longer than the production\ntwo-second disconnect\ntimeout, so loss-bearing nightly real-UDP peers now opt into a test-only\n10 s session timeout\nvia `--disconnect-timeout-ms`. Loss-free nightly rows (including the\ndeliberate 2 s\nstaggered-startup disconnect/retry test) and the per-PR smoke tier keep\nthe production default,\nand production-timeout semantics remain covered by deterministic\nin-process suites.\n- `PLAN.md` drops the completed P1 item, leaving only the future P2\nsearch experiment.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change\n- [x] 📚 Documentation\n- [ ] ♻️ Refactor\n- [x] 🧪 Test\n- [x] 🔧 CI/Build\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] Zero-panic policy: no production code added; fixture helpers\nreturn `Result`, and test-only\n      peer parsing uses the existing harness conventions.\n- [x] Added tests that prove the evidence is real (ring-cycle counts,\nrequest/state digests,\ncanonical artifact bytes, full oracle replay identity,\ncorrupted-checksum control).\n- [x] `cargo fmt` and `cargo clippy --workspace --all-targets --features\ntokio,json -- -D warnings`\n      pass.\n- [x] `cargo nextest run --features tokio,json` passes (3,121 passed, 74\nskipped).\n\n### If Applicable\n\n- [x] Documentation updated (production checklist + wiki mirror + PLAN).\n- [ ] `CHANGELOG.md`: no public API or user-observable behavior changed;\ntest/CI/docs only.\n- [ ] Examples: not affected.\n- [x] No new compiler warnings.\n\n## Testing\n\n**Tests added/modified:**\n\n- `fortress-replay-fixture`: extended capture/playback evidence,\nrequest-stream digest,\ncanonical-artifact check, ring-coverage and checksum-coverage\nassertions.\n- Browser smoke and Godot probes assert the new request digest and\n4,096-frame evidence.\n-\n`simulation::corpus_replay::checked_in_extended_ring_schedule_recycles_rings_and_replays_identically`.\n- Nightly peer binary `--disconnect-timeout-ms` parsing and session\napplication.\n\n**Manual testing performed:**\n\n- `cargo nextest run --features tokio,json --no-capture` — 3,121 passed,\n74 skipped.\n- `cargo nextest run --features hot-join,tokio,json --no-capture` —\n3,385 passed, 75 skipped.\n- Browser lane locally against Chrome for Testing 157.0.8103.78 — 5/5\npassed, including the\n  extended replay and both tab-suspension tests.\n- Safe-release corpus replay with `-C overflow-checks=on -C\ndebug-assertions=on` — 12 passed.\n- Full 47-test nightly real-UDP suite in release with the host-stall\ntolerance applied — passed.\n- Capture determinism: two captures are byte-identical; artifact is\n167,998 bytes and replays with\n  zero desyncs.\n- Agent preflight, links, doc claims, wiki consistency, plan hygiene,\ntypos, markdownlint,\n  actionlint, shell portability, and network timing invariants pass.\n\n## Related Issues\n\nCloses #390.",
          "timestamp": "2026-11-11T22:17:01-08:00",
          "tree_id": "9c6bf70a9edc272bd3c4c29a56cdb22184ef07ff",
          "url": "https://github.com/wallstop/fortress-rollback/commit/0b76a47782d8a44e88ccab298536c60e85519438"
        },
        "date": 1794464692773,
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
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 68,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 233,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 24,
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
            "value": 44,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 123,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 448,
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
            "value": 70,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 88,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 121,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 120,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 162,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 246,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 224,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 308,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 484,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 117,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 133,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 171,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 220,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 260,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 366,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 418,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 508,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 695,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 275,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 369,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 525,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 671,
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
            "value": 110,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 337,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 535,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 759,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 30,
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
            "value": 67,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 266,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1061,
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
          "id": "caa243eb42e7ace805f284fec2e9ef6e5b7fbec7",
          "message": "test(simulation): make rare-state census non-vacuous (P2) (#404)\n\n## Description\n\nThe nightly sometimes-state census was stable but non-vacuous for only\n2/5 IDs. Ten consecutive hosted windows (2026-11-03 through 2026-11-12)\nreported the floor and nudge rows at stable rates (620-630 and 893-902\nper 1,000 schedules) while prediction-limit, sparse-earlier, and\ninput-ring stayed at zero. The default nightly session uses a 128-slot\nring, `max_prediction = 8`, `input_delay = 1`, and\n`SaveMode::EveryFrame`, so two of those rows were structurally\nunreachable. Issue #316 requires at least 3/5 eligible IDs, so the\noriginal organic presence ratchet is rejected.\n\nThis change makes the census non-vacuous without changing the fault mix\nfor the existing 96% of rows:\n\n- Add an optional, serde-defaulted `SimConfig::input_queue_length` axis\nand wire it into the normal and hot-join replacement builders.\n- Reserve 40/1,000 nightly rows for a two-slot ring with `max_prediction\n= 1` and `input_delay = 0`, using `seed % 100` residues that are\nhistorically `Clean`, never `Swarm`, and span 2- through 16-player\nmeshes.\n- Validate the effective ring (including the 128-slot default) against\n`input_delay + max_prediction`, reject every `SetInputDelay` that would\nexceed it, and require schema 20 for an explicit override.\n- Fix the shrink regression fixture that combined a 127-frame delay with\nan eight-frame prediction window, which session construction rightly\nrejects.\n- Update `PLAN.md` to the next P2 step: collect seven post-redesign\nwindows and add the presence ratchet if at least 3/5 IDs remain stable.\n\nA local 1,000-schedule window reports `prediction=40`, `floor=617`,\n`nudge=897`, `sparse-earlier=0`, `input-ring=40` over 5,000,000 steps\nand 9,025 peer sessions (4/5 eligible). The remaining sparse-earlier gap\nis tracked in #402; the ratchet scope is #403.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n-\n`nightly_rare_state_coverage_cell_is_present_and_exercises_ring_and_prediction_states`\nchecks the 40-row share, per-shard presence, 2- and 16-player edges,\nschedule shape, full-oracle pass, both state hits, and exact replay\nidentity.\n- Schedule validation tests cover ring bounds, retained-window overflow\non explicit and default rings, `SetInputDelay` overflow, schema floor,\nand legacy serialization omission.\n- Full nightly window locally: 8/8 shards, 1,000 schedules / 5,000,000\nsteps; targeted probes 4/4.\n\n**Manual testing performed:**\n\n- `cargo nextest run --features tokio,json --no-capture` — 3,124 passed,\n74 skipped.\n- `cargo nextest run --features hot-join,tokio,json --no-capture` —\n3,388 passed, 75 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings` — clean.\n- `cargo test --release --test simulation --features hot-join\n'simulation::fleet::'` — 76 passed.\n- `cargo test --release --test simulation --features hot-join\n'simulation::harness::schedule::tests::'` — 39 passed.\n- `python3 scripts/ci/agent-preflight.py --auto-fix`, plan hygiene,\nmarkdownlint, typos, link checks, and version sync — clean.\n\n## Related Issues\n\nRelates to #316 (original census contract), #402 (sparse-earlier organic\ngap), and #403 (seven-window ratchet scope).",
          "timestamp": "2026-11-12T09:05:00-08:00",
          "tree_id": "ba92c7150ff8cde3273ac2c8fde829241e648ba6",
          "url": "https://github.com/wallstop/fortress-rollback/commit/caa243eb42e7ace805f284fec2e9ef6e5b7fbec7"
        },
        "date": 1794503564146,
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
            "value": 26,
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
            "value": 232,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 25,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 31,
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
            "value": 121,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 448,
            "range": "± 4",
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
            "value": 70,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 87,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 118,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 120,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 166,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 254,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 223,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 313,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 486,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 117,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 132,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 170,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 219,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 261,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 364,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 417,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 510,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 709,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 273,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 377,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 511,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 665,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 89,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 119,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 338,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 544,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 768,
            "range": "± 18",
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
            "value": 266,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1062,
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
          "id": "ec9ac03fbe84acca7ea79bd727302ce4ef3a3c7a",
          "message": "test(simulation): add seven-window organic presence ratchet (P2) (#405)\n\nRecord the sparse-earlier sometimes-state as targeted-only and add the checked-in seven-window evaluator required by #403. Validated against seven scheduled-main artifacts: pre-redesign baseline rejects at 2/3 required eligible IDs.",
          "timestamp": "2026-11-12T10:27:50-08:00",
          "tree_id": "ef3e9f081e05b8b2bfc628c35278597e9dc6ed6c",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ec9ac03fbe84acca7ea79bd727302ce4ef3a3c7a"
        },
        "date": 1794508630636,
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
            "value": 27,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 67,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 230,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 46,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 124,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 467,
            "range": "± 9",
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
            "value": 23,
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
            "value": 72,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 91,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 122,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 120,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 167,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 243,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 223,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 308,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 473,
            "range": "± 2",
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
            "value": 155,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 183,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 221,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 271,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 339,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 421,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 527,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 678,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 274,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 364,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 528,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 693,
            "range": "± 10",
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
            "value": 110,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 339,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 550,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 766,
            "range": "± 17",
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
            "value": 69,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 259,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1020,
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
          "id": "83013d983262e36ee568c9bf49efa8bf265ad183",
          "message": "test: satisfy strict clippy 1.100 lints (#406)\n\nThe stable toolchain moved to 1.100.0 between the PR and main runs; fix every new redundant-clone and needless-range-loop hit in test code so the strict clippy gates stay green.",
          "timestamp": "2026-11-12T11:22:08-08:00",
          "tree_id": "b95d0e166195c42b230baa4150f1a3392532b102",
          "url": "https://github.com/wallstop/fortress-rollback/commit/83013d983262e36ee568c9bf49efa8bf265ad183"
        },
        "date": 1794511871910,
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
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 40,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 57,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 154,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 524,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 99,
            "range": "± 3",
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
            "value": 162,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 176,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 241,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 357,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 337,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 463,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 692,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 175,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 221,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 266,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 341,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 411,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 532,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 644,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 769,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1030,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 387,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 514,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 707,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 923,
            "range": "± 31",
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
            "value": 418,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 680,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 939,
            "range": "± 26",
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
            "value": 317,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1452,
            "range": "± 9",
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
        "date": 1794552781450,
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
            "value": 27,
            "range": "± 1",
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
            "value": 231,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 26,
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
            "value": 46,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 125,
            "range": "± 5",
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
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 91,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 120,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 121,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 170,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 254,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 228,
            "range": "± 8",
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
            "value": 486,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 129,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 156,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 172,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 232,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 286,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 347,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 420,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 530,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 682,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 276,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 395,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 543,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 699,
            "range": "± 45",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 90,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 119,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 347,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 551,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 773,
            "range": "± 10",
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 69,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 259,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1020,
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
          "id": "3db1458d59ae7fe18431e1192f16820bc10ec94a",
          "message": "test(simulation): make violation-site exercise census exact (#413)\n\n## Summary\n\nCloses #411.\n\nThe nightly violation-site exercise census joined the content-addressed\nproduction\n`report_violation!` baseline with Cobertura line coverage and fleet\nshards. Coverage is only a\nconservative lower bound for macro invocations: LLVM can attribute an\nexpansion to surrounding\nlines, so genuinely executed assertions stayed in the unexercised set.\nThis change adds an exact\nruntime source and folds it into the nightly report.\n\n## Changes\n\n- **Opt-in `violation-census` feature.** `SpecViolation::new` calls a\ncold-path hook that records\neach new `file:line` location in a `parking_lot`-guarded `BTreeSet` (256\nlocations per process).\nWhen `FORTRESS_VIOLATION_CENSUS_DIR` is set, each new location rewrites\na bounded\n`observations-<pid>.json` snapshot with sorted unique entries, an\noptional workflow `run_id`, and\na `truncated` marker when the 256-location or 32 KiB limit drops\nentries. The hook compiles out\nof normal builds and Loom builds, never panics, and treats every\nfilesystem error as diagnostic.\n- **Exact observations job.** The nightly workflow runs the full suite\nwith\n`--features tokio,json,hot-join,violation-census` and uploads the\nbounded artifacts.\n- **Census join.** `violation_census.py` gained `--observations-dir`,\n`--expected-run-id`, and\n`--partial` with artifact-count, per-artifact size, per-artifact\nlocation, and processed-location\ncaps plus schema, run-id, process-id, filename, and order validation.\nAbsolute and Windows paths\nnormalize without promoting test-local `src/` labels. Test-only labels\ncount as unmatched;\nmalformed `src/` locations still fail loudly; unmatched source and\nignored entries are surfaced\n  in the report. Empty fleet or observation inputs are rejected.\n- **Failure behavior.** The census runs with `if: always()`, tolerant\nartifact downloads, and marks\nthe report `partial` when a producer job or either artifact download\nfails.\n- Documented the feature in `Cargo.toml`, the user guide, migration\nguide, wiki mirrors, and\nchangelog, and added it to the cargo-geiger and strict-clippy feature\ncensuses.\n- Fixed a pre-existing non-line `SpecViolation` location on the\nscoped-observer drop path so\n  diagnostic consumers and the census can parse it.\n- Updated the P2 plan item to the exact-source next step.\n\n## Evidence\n\n- Full suite with the hook: **3,399 passed, 76 skipped**; 297 artifacts,\n163 observed locations,\n  max 5 per artifact, 0 over 64 KiB, one run id.\n- Census against those observations: **109/268 exercised** (vs. 25/268\nfrom coverage alone),\n109 matched, 54 unmatched (41 under `src/` surfaced for review), 0\ntruncated.\n  `src/network/compression.rs` no longer appears in the unexercised set.\n- Downloaded the latest main coverage artifact (`fe0b935`, run\n43380911481): coverage-derived IDs\n`0`, exact-observation IDs `109`; the exact source is currently a strict\nsuperset.\n- `cargo clippy --workspace --all-targets --features\ntokio,json,violation-census -- -D warnings`:\nclean. `cargo check --no-default-features --features violation-census`:\nclean. Both census tests\n  pass, including snapshot replacement and byte-budget truncation.\n- `python3 -m pytest scripts/tests/ -q`: **2,227 passed**; `mypy`:\nclean.\n- `ruff check`/`ruff format --check`, `actionlint`,\n`check-plan-hygiene`,\n`check-changelog-unreleased`, `sync-version --check`, `check-links`,\n`check-doc-claims`,\n  `check-unbounded-alloc`, and `agent-preflight.py --auto-fix`: clean.\n\nNo public API change; the hook is feature-gated and absent from normal\nbuilds.",
          "timestamp": "2026-11-13T19:59:28-08:00",
          "tree_id": "5a95a3c37f2b3175435b146422036faa597e0b1b",
          "url": "https://github.com/wallstop/fortress-rollback/commit/3db1458d59ae7fe18431e1192f16820bc10ec94a"
        },
        "date": 1794629218075,
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
            "value": 230,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 26,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 32,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 47,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 125,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 458,
            "range": "± 14",
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
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 91,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 119,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 121,
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
            "value": 252,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 224,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 314,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 485,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 129,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 155,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 183,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 233,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 290,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 365,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 419,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 530,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 694,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 269,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 367,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 510,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 675,
            "range": "± 3",
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
            "value": 110,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 336,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 543,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 776,
            "range": "± 13",
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 69,
            "range": "± 0",
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
            "value": 1019,
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
          "id": "482ec27e9e6473f31d3bd08451181b2a5b79258f",
          "message": "test(simulation): complete and cut the exact violation-site census (#414)\n\nInventory report_violation_to! sites, wrapper-macro call sites, and placeholder-first messages; regenerate the 284-site baseline; delete two redundant receive-path asserts; cover 36 previously unexercised sites with focused tests. Local exact census: 154/284 exercised (130 unexercised), down from 159 unexercised in nightly run 43555370869.",
          "timestamp": "2026-11-13T21:23:36-08:00",
          "tree_id": "962e69a2812ca55ab94cdfcfc4361ee3dab7a3bd",
          "url": "https://github.com/wallstop/fortress-rollback/commit/482ec27e9e6473f31d3bd08451181b2a5b79258f"
        },
        "date": 1794634305969,
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
            "range": "± 2",
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
            "value": 33,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 40,
            "range": "± 1",
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
            "value": 151,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 524,
            "range": "± 27",
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
            "value": 99,
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
            "value": 166,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 176,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 254,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 366,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 369,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 492,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 731,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 175,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 205,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 251,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 362,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 430,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 561,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 652,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 780,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1051,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 389,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 507,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 703,
            "range": "± 54",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 941,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 94,
            "range": "± 0",
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
            "value": 400,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 646,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 935,
            "range": "± 28",
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
            "value": 313,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1417,
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
          "id": "e64f23ad8d258f0c006d3789d7dcde04fba3d61a",
          "message": "test: cover reachable violation-site guards (P2) (#415)\n\nExercise the reachable defensive guards behind the remaining unexercised violation-site inventory. Local exact census moves 154/284 -> 202/284 exercised (130 -> 82 unexercised). Test-only change; no production behavior.",
          "timestamp": "2026-11-13T22:56:20-08:00",
          "tree_id": "e6c74af16e17544ca19a2200197803cc67e25420",
          "url": "https://github.com/wallstop/fortress-rollback/commit/e64f23ad8d258f0c006d3789d7dcde04fba3d61a"
        },
        "date": 1794639878921,
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
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/16",
            "value": 31,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 87,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 328,
            "range": "± 3",
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
            "value": 36,
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
            "value": 139,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 529,
            "range": "± 4",
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
            "value": 91,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 112,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 147,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 160,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 208,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 297,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 299,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 398,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 616,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 147,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/8",
            "value": 167,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 198,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 299,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 344,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 409,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 566,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 663,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 826,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 425,
            "range": "± 13",
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
            "value": 587,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 753,
            "range": "± 12",
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
            "value": 115,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 503,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 864,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1264,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 24,
            "range": "± 5",
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
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 297,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1165,
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
          "id": "b9ce93840086253f65a0123a8ac1bbea66899ed2",
          "message": "test(simulation): cut the residual violation-site census (P2) (#417)\n\n## Description\n\nExercise the remaining reachable defensive `report_violation!` guards\nbehind the exact violation-site census, completing stage 2 of the\nfleet-search violation-coverage item. Local exact observations move from\n**202/284 to 243/284 exercised** (82 -> 41 unexercised). Test-only\nchange; no production behavior changed.\n\nThe 41 residual sites are classified in #416:\n\n- 19 allocation/encode/writer fallbacks with no fault-injection hook.\n- 14 unreachable-by-construction postconditions / duplicate guards\n(deletion needs maintainer-approved baseline regeneration).\n- 8 reachable defensive fallbacks needing targeted fault injection\n(snapshot capture serialization, replay recorder failure, endpoint\nre-arm, propagated/timed-out disconnect application).\n\n`PLAN.md` now hands the residual classification to #416 and keeps the\ncalendar-bound seven-window ratchet item.\n\n## Type of Change\n\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly (`PLAN.md` only; no\nuser-facing docs or changelog entry for test-only work)\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n(not applicable: test-only)\n- [ ] I have updated relevant examples in the `examples/` directory (not\napplicable)\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `src/sync_layer/mod.rs`: unaddressable `save_current_state` frame\nfallback; reactivated-slot `synchronized_inputs` pre-activation bound.\n- `src/input_queue/mod.rs`: gap-fill missing replication source;\n`reset_to_frame` delayed-frame underflow.\n- `src/sessions/p2p_spectator_session.rs`: missing host connect-status\nrow; canonical commit missing input slot/status.\n- `src/sessions/p2p_session.rs`: missing synchronized inputs;\nout-of-window hot-join activation; staged 2-peer and N-peer serve\nfailure paths; N-peer abort-before-capture; joiner backfill guards;\nremote disconnect snapshot fallbacks; staged survivor pending\nreactivation; constructor guards; queued coordinated-drop guard;\ntruncated spectator batch; 2-peer joiner apply failure; N-peer buffered\njoiner coordinator loss/apply failure.\n- `src/network/protocol/mod.rs`: armed failing `Config::Input`\nserializer for the `enqueue_replicated_input` serialization fallback.\n\n**Manual testing performed:**\n\n- `cargo +1.100.0 fmt --check` (CI-matching toolchain): clean.\n- `cargo +1.100.0 clippy --workspace --all-targets --features\ntokio,json,hot-join,violation-census -- -D warnings`: clean.\n- `cargo +1.100.0 clippy --workspace --all-targets -- -D warnings`:\nclean.\n- `cargo +1.100.0 nextest run --profile ci --features\ntokio,json,hot-join,violation-census`: 3467 passed, 76 skipped.\n- `cargo +1.100.0 nextest run`: 3141 passed, 75 skipped.\n- Exact census: 284 baseline, 243 exercised, 41 unexercised.\n- `python3 scripts/ci/agent-preflight.py --auto-fix`: all checks passed.\n\n## Related Issues\n\n- Relates to #416\n- Relates to #403",
          "timestamp": "2026-11-14T10:37:01-08:00",
          "tree_id": "8f629670cbf2ea972ec98d6ecf806dfbb3b5f699",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b9ce93840086253f65a0123a8ac1bbea66899ed2"
        },
        "date": 1794681922877,
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
            "value": 34,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/64",
            "value": 82,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/zeros/256",
            "value": 274,
            "range": "± 14",
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
            "value": 46,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/16",
            "value": 59,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/64",
            "value": 156,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 526,
            "range": "± 15",
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE decode/zeros/256",
            "value": 27,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/8",
            "value": 98,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 126,
            "range": "± 3",
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
            "value": 175,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 249,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 367,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 338,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 463,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 692,
            "range": "± 6",
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
            "value": 225,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/8",
            "value": 265,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 364,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 432,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 562,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 644,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 766,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 1014,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 395,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 538,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 724,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 985,
            "range": "± 17",
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
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 409,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 660,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 937,
            "range": "± 33",
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
            "value": 3,
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1454,
            "range": "± 9",
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
        "date": 1794688339730,
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
            "value": 230,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/4",
            "value": 25,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/8",
            "value": 32,
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
            "value": 124,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "RLE encode/random/256",
            "value": 464,
            "range": "± 7",
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
            "value": 72,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/8",
            "value": 92,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/8",
            "value": 120,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/16",
            "value": 123,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/16",
            "value": 168,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/16",
            "value": 247,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_4b/32",
            "value": 224,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_4b/32",
            "value": 308,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_4b/32",
            "value": 486,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/8",
            "value": 119,
            "range": "± 1",
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
            "value": 172,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/16",
            "value": 221,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/16",
            "value": 270,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/16",
            "value": 349,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/idle_encode_8b/32",
            "value": 423,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/active_encode_8b/32",
            "value": 529,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Compression pipeline/fighting_encode_8b/32",
            "value": 698,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/idle",
            "value": 280,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/active",
            "value": 385,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/fighting",
            "value": 530,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Compression ratio analysis/roundtrip/analog",
            "value": 691,
            "range": "± 92",
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
            "value": 110,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 333,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 552,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 762,
            "range": "± 12",
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
            "value": 20,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=4",
            "value": 69,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=8",
            "value": 259,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "H-16P confirmed_frame/steady_mesh/N=16",
            "value": 1019,
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
      }
    ]
  }
}