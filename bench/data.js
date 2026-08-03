window.BENCHMARK_DATA = {
  "lastUpdate": 1785740382236,
  "repoUrl": "https://github.com/wallstop/fortress-rollback",
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
          "id": "23b8593f89454125e02b5cf71291f8723186c55d",
          "message": "[codex] Hardening M3 LegacyDisconnect lifecycle op (#207)\n\n## Summary\n\nAdds the M3 `LegacyDisconnect` lifecycle operation to the deterministic\nsimulation harness.\n\n- bumps simulation schedule schema to v6 and serializes\n`ScheduleEvent::LegacyDisconnect { by, target }`\n- wires the runner to call the real `P2PSession::disconnect_player` API,\nretiring/detaching the target only on success\n- adds planted coverage for serialization, malformed event validation,\ndeterministic execution, and the current Halt/D13 non-recovery shape\n- adds a pre-retirement divergence control so a legacy-disconnected\ntarget cannot hide a determinism bug behind the alive mask\n\nThis is intentionally not modeled as graceful convergence: the legacy\npath remains Halt/D13-facing and the test pins non-recovery precisely.\n\n## Validation\n\n- `cargo fmt`\n- `cargo nextest run --no-capture --no-fail-fast legacy_disconnect\npre_disconnect lifecycle_events_round_trip_through_json`\n- `cargo nextest run --no-capture --test simulation` (126 passed, 10\nskipped)\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- `cargo nextest run --no-capture` (2395 passed, 57 skipped)\n- `cargo nextest run --features hot-join --no-capture` (2634 passed, 57\nskipped)\n\n## Review Notes\n\nA local adversarial review found and fixed an overly loose\nexpected-failure assertion: the test now pins the exact live-peer\nnon-recovery set and only the expected failure classes.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes are confined to simulation harness, schedule schema, and\ntests; production session code is only invoked, not modified.\n> \n> **Overview**\n> Introduces **`ScheduleEvent::LegacyDisconnect`** and bumps the\nsimulation schedule schema to **v6**, so corpus schedules can plant the\nlegacy **`P2PSession::disconnect_player`** path alongside existing\nlifecycle ops.\n> \n> The harness runner now executes that event like **`GracefulRemove`**:\none peer calls the real API, retires and detaches the target on success,\nand records **`SessionError`** on failure. **`LegacyDisconnect`** shares\nthe same up-front validation as graceful remove (range checks,\nremote-only target). The oracle treats legacy-disconnected peers like\nother retired peers for the alive mask (liveness excluded,\npre-retirement state still compared).\n> \n> **`clean_four_peer_lifecycle_schedule`** deduplicates the 4-peer mesh\nsetup used by peer-kill, graceful-remove, and legacy-disconnect\nbuilders; legacy schedules use **`DropPolicy::Halt`** to match today’s\nhalt-oriented behavior.\n> \n> New fleet coverage documents that this is **not** a\ngraceful-convergence contract:\n**`legacy_disconnect_reports_halt_non_recovery`** expects a failing run\nwith precise oracle failure classes, **`recovered_within_b == false`**,\nand deterministic trace hashes; plus pre-disconnect divergence and\nmalformed-event rejection tests. JSON round-trip tests include the new\nevent.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n9eb312a0acec2f1d1b443384e419c04e7c2f44ca. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-06T13:47:21-07:00",
          "tree_id": "be71267422236758ffb3c973962535ee94e7d524",
          "url": "https://github.com/wallstop/fortress-rollback/commit/23b8593f89454125e02b5cf71291f8723186c55d"
        },
        "date": 1783371111064,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 121,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 166,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 510,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 779,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1115,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 127293,
            "range": "± 3413",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 48570,
            "range": "± 253",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1405,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1603,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "b877befafef0f289321bc83f26fa6108fbfbb88d",
          "message": "Add freeze-frame convergence oracle (#208)\n\n## Summary\n- Record simulation harness applied `(input, InputStatus)` vectors by\nsimulated frame with rollback last-write-wins semantics.\n- Add an always-on oracle check for retired-slot freeze-frame\nconvergence across live `Running` survivors, including stable\nframe/value comparison and missing-freeze diagnostics.\n- Add negative controls for divergent freeze values, divergent freeze\nstarts, mixed `Some`/`None`, non-`Running` live peers, all-missing\nfreezes, missing-slot resets, value-change resets, and speculative tail\nbounds.\n\n## Validation\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- `cargo nextest run --no-capture --test simulation` - 135 passed, 10\nskipped\n- `cargo nextest run --no-capture` - 2404 passed, 57 skipped\n- `cargo nextest run --features hot-join --no-capture` - 2643 passed, 57\nskipped\n\n## Review\n- Adversarial sub-agent found one diagnostic precision issue in mixed\n`None`/`Some` order.\n- Fixed with `any_stable_freeze` gating and\n`oracle_does_not_report_missing_when_later_survivor_has_freeze_frame`.\n- Copilot found the divergence comparison should use the same live\n`Running` survivor set as missing-freeze diagnostics.\n- Fixed by iterating over `live_running_peers` and adding\n`oracle_ignores_non_running_live_peer_for_freeze_frame_comparison`.\n- Follow-up adversarial review reported zero issues.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes are confined to simulation test harness and oracle validation;\nproduction rollback/session code is untouched.\n> \n> **Overview**\n> Adds **(e) freeze-frame convergence** to the simulation harness\noracle: when a player slot retires mid-run, every live **`Running`**\nsurvivor must agree on the stable frame and frozen input value where\nthat slot begins presenting **`InputStatus::Disconnected`**.\n> \n> The harness **`SimGameStub`** now records per-simulated-frame applied\n`(input, InputStatus)` vectors with rollback **last-write-wins**\nsemantics (same as recorded state), and the runner feeds those maps into\n**`finalize_with_applied_inputs`**. Older unit tests keep calling\n**`finalize`**, which leaves (e) inert via an empty applied-inputs\nslice.\n> \n> The oracle compares survivors only within each peer’s confirmed prefix\n(ignoring speculative disconnected tails), derives the freeze point from\nthe final stable trailing **`Disconnected`** run (value or missing-slot\nchanges reset it), and reports **`FreezeFrameDivergence`** or\n**`FreezeFrameMissing`** when appropriate. **`any_stable_freeze`**\navoids mis-firing the all-missing diagnostic when only the iteration\norder differs in mixed `None`/`Some` cases.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\na3f19e0c259a22b83a53cbfdafa6731d3245bba5. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-06T19:20:27-07:00",
          "tree_id": "175f39e0807ade3c1900c09a45983e2a5628c957",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b877befafef0f289321bc83f26fa6108fbfbb88d"
        },
        "date": 1783391070634,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 62,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 89,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 341,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 580,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 862,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 50744,
            "range": "± 432",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 22038,
            "range": "± 365",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 450,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 560,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "b9db9ef7b5026239b6e8d1b0514da79d67612463",
          "message": "[codex] Wire nightly baseline sweep (#209)\n\n## Summary\n\n- Wire the deterministic full baseline sweep into the nightly network\nworkflow on Linux.\n- Add N=3/8/12/16 scale rows to the ignored full matrix and pin the\nexact row set with a cheap PR test.\n- Strengthen the full-matrix sweep to assert the same health invariants\nas the PR gate, and write sweep artifacts under `$RUNNER_TEMP` so\nuploads cannot publish stale cached `target` data.\n- Update locked dev dependencies (`crossbeam-epoch`, `rand`) to patched\nversions after the security advisory gate failed on newly published\nadvisories.\n\n## Validation\n\n- `cargo nextest run --no-capture --test simulation -E\n'test(full_matrix_includes_scale_spot_rows) | test(sweep_pr_gate)'`\n- `FORTRESS_SWEEP_OUT=/tmp/fortress-full-matrix.jsonl\nFORTRESS_SWEEP_GIT_SHA=local-test-sha cargo nextest run --profile\nci-network-nightly --release --run-ignored ignored-only -E\n'test(full_matrix_sweep)' --test simulation --no-capture`\n- `actionlint .github/workflows/ci-network-nightly.yml`\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `cargo deny check advisories`\n- `cargo deny check`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes are confined to CI orchestration, simulation test/baseline\ncapture, and non-runtime dependency lockfile updates—no production\nnetworking or auth paths.\n> \n> **Overview**\n> The **nightly network workflow** on Linux now runs the ignored\n`full_matrix_sweep` simulation test, writes `full-matrix.jsonl` under\n`$RUNNER_TEMP/fortress-sweep` (avoiding stale cached `target`\nartifacts), uploads it for 30 days, and **fails the job** if that step\ndid not succeed—even though the sweep step itself uses\n`continue-on-error` so the artifact can still upload.\n> \n> **Baseline sweep harness** changes: `FORTRESS_SWEEP_GIT_SHA` is read\nat runtime via `std::env::var` instead of compile-time `option_env!`.\nPR-gate and full-matrix sweeps share **`assert_cell_health`**. The full\nmatrix gains four **scale spot rows** (3/8/12/16 players, regional\nprofile); a cheap PR test **`full_matrix_includes_scale_spot_rows`**\npins 68 cells total. Docs no longer defer nightly CI for the full\nmatrix.\n> \n> **`Cargo.lock`**: bumps `crossbeam-epoch` and `rand` to address\nadvisory gate failures.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n2148017f4997d7f8fac2e22974b7d78e83a0bbde. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-06T21:45:52-07:00",
          "tree_id": "fb7f89c67aba8553dc37aae23b56ffd62adc1156",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b9db9ef7b5026239b6e8d1b0514da79d67612463"
        },
        "date": 1783399856506,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 114,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 163,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 462,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 708,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1048,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 130839,
            "range": "± 7519",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 44025,
            "range": "± 337",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1243,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 101",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "c43fd3df301c3e268d1a89214e915c97c4d6efdd",
          "message": "Advance M3 simulation hardening (#210)\n\n## Summary\n- add simulation spectator convergence oracle support\n- add preplanned spectator harness path and negative controls\n- keep M3 plan/progress current locally\n\n## Validation\n- `cargo fmt`\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `cargo test --test simulation spectator -- --nocapture`\n- `cargo test --test simulation -- --nocapture`\n- `cargo nextest run --no-capture`\n- `cargo nextest run --features hot-join --no-capture`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Large changes to simulation oracle and harness CI gates (spectator\ninvariants, fingerprint semantics, blessed sweep baselines); scope is\ntest infrastructure, not production session code in this diff.\n> \n> **Overview**\n> Extends the simulation harness for **M3 §6.2**: preplanned redundant\nspectators (`SimConfig::spectator_hosts`), a `SpectatorSession` drive\npath, and oracle checks that displayed inputs and `Disconnected` slots\nmatch the live mesh canon—including a **display-frame** post-drop floor\n(not schedule steps) with fleet regressions and negative controls.\n> \n> Generalizes the runner over a **`SimInput`** trait (`StubInput` /\n**`WideStubInput` 32B**) via `run_with_input`, compares inputs with\n**`InputFingerprint`** (full serialized bytes), and adds reviewed\n**violation allowlist** plumbing plus **`RunReport::violation_census`**\nand an ignored 200-seed Error+ census test.\n> \n> The **baseline sweep** now runs PR gate and full matrix cells at **4B\nand 32B**, bumps report schema/labels, and replaces **`sweep-v1.json`**\nwith **`sweep-v2.json`** (10 gate rows).\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n62a905a362a1893a34cbd1a2469039acbffab408. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-07T12:30:46-07:00",
          "tree_id": "d1a03e614f11758b804e3efb53607026bc614625",
          "url": "https://github.com/wallstop/fortress-rollback/commit/c43fd3df301c3e268d1a89214e915c97c4d6efdd"
        },
        "date": 1783452918798,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 113,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 162,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 461,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 700,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1053,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 128870,
            "range": "± 578",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 43684,
            "range": "± 3552",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "a3776b43fabddfd4af678fde3e4518621e167424",
          "message": "Add spectator host kill simulation coverage (#211)\n\n## Summary\n\n- Add `ScheduleEvent::SpectatorHostKill { host }` to the deterministic\nsimulation lifecycle vocabulary and bump the schedule schema to v7.\n- Reuse the lifecycle retirement path for spectator host kills and\nvalidate malformed schedules without predicting runtime-contingent API\ndrops as guaranteed retirements.\n- Add planted fleet coverage for partition-only fail-closed behavior\nversus partition + spectator-host-kill recovery, with deterministic\ntrace reproduction and final spectator-host compaction assertions.\n\n`PLAN.md` and `progress/**` remain local ignored planning artifacts and\nare not part of this PR.\n\n## Validation\n\n- `cargo fmt --check`\n- `cargo nextest run\nspectator_failover_survives_configured_host_kill_under_partition\nrun_rejects_malformed_spectator_host_kill_events\nlifecycle_events_round_trip_through_json --no-capture --no-fail-fast`\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- `git diff --check`\n- `cargo nextest run --no-capture` (2432 passed, 58 skipped)\n- `cargo nextest run --features hot-join --no-capture` (2671 passed, 58\nskipped)\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes are confined to the simulation harness and tests; production\nsession behavior is only exercised indirectly through existing\ncrash/retire semantics.\n> \n> **Overview**\n> Adds **`ScheduleEvent::SpectatorHostKill`** to the deterministic\nsimulation lifecycle vocabulary and bumps the schedule schema to **v7**,\nso corpus schedules can express a crash of a peer that must be listed in\n`spectator_hosts`—separate from generic `PeerKill`.\n> \n> The harness **retires peers through a shared\n`retire_peer_for_lifecycle` path** (detach, oracle dead-mark, spectator\nfloor update) for kills, graceful/legacy drops, and the new event.\n**`RunReport` gains `spectator_final_hosts`** (end-of-run redundant host\ncount from `SpectatorSession::num_hosts()`), including in `expect_pass`\nrepro output.\n> \n> **Up-front validation** rejects out-of-range hosts, hosts not in\n`spectator_hosts`, and `SpectatorHostKill` after an earlier guaranteed\nkill (`PeerKill` / prior `SpectatorHostKill`); **`GracefulRemove` /\n`LegacyDisconnect` are not treated as guaranteed retirements** at\nvalidate time because they can be runtime no-ops.\n> \n> **Fleet coverage** plants partition + host-kill scenarios\n(partition-only fails spectator oracle; kill recovers with two hosts and\ndeterministic traces) plus negative tests for malformed schedules and\nJSON round-trip of the new event.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nb886014c4abe2d1965b6a60c5a22f0558f478d55. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-07T14:00:33-07:00",
          "tree_id": "38ccf1ee84641ce61266e9140e1c2e23ca96f9f9",
          "url": "https://github.com/wallstop/fortress-rollback/commit/a3776b43fabddfd4af678fde3e4518621e167424"
        },
        "date": 1783458296828,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 118,
            "range": "± 2",
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
            "value": 478,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 755,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1111,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 126366,
            "range": "± 895",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 47369,
            "range": "± 255",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1405,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1602,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "bd69b090f97b0b582b2a36f9116f60ca8349d16f",
          "message": "Add hot-join lifecycle simulation op (#212)\n\n## Summary\n\nAdds the M3 `ScheduleEvent::HotJoin { slot }` lifecycle operation to the\ndeterministic simulation harness.\n\n- Bumps the simulation schedule schema to v8 and round-trips `HotJoin`\nevents.\n- Wires hot-join schedules through the public `start_hot_join_session`\npath with a deterministic coordinator, fresh `SimSocket`, and explicit\nfeature/config validation.\n- Models hot-joined slots as replacement generations instead of\npermanent dead-mask entries.\n- Adds regressions proving the clean-drop returning-slot path, fail-loud\nbehavior after prior runtime retirement, and settled pre-handoff oracle\ncoverage.\n\n## Validation\n\n- `cargo fmt`\n- `cargo nextest run --features hot-join --no-capture\nhot_join_reactivates_cleanly_dropped_slot > /tmp/hotjoin-sim.txt 2>&1`\n- `cargo nextest run --features hot-join --no-capture simulation::fleet\n> /tmp/hotjoin-fleet.txt 2>&1` (54 passed)\n- `cargo test --test simulation --no-run > /tmp/simulation-no-run.txt\n2>&1`\n- `cargo clippy --workspace --all-targets --features tokio,json,hot-join\n> /tmp/clippy-hotjoin.txt 2>&1`\n- `python3 scripts/ci/agent-preflight.py --auto-fix >\n/tmp/agent-preflight.txt 2>&1`\n\n## Notes\n\n`PLAN.md` and `progress/session-83-m3-hotjoin-lifecycle-op.md` are\nupdated locally, but both paths are ignored by this repository's\n`.gitignore`, so they are not included in the PR diff.\n\nResiduals recorded locally: slot-0 fixed-order and N-peer hot-join DST\ncoverage remain future census/random-generator work.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Changes lifecycle orchestration and oracle canonical-input semantics\nfor hot-join; impact is mostly confined to simulation tests behind the\n`hot-join` feature, but mistakes could mask real determinism or handoff\nbugs.\n> \n> **Overview**\n> Introduces **`ScheduleEvent::HotJoin`** (schedule schema **v8**) and\nruns it through the simulation harness via **`start_hot_join_session`**,\nwith **`with_hot_join(true)`** on coordinator peers and static\nvalidation (feature flag, `input_delay == 0`, `max_prediction >= 1`).\n> \n> Hot-join is modeled as a **replacement generation** at the same\nslot—not a permanent dead-peer mask: the oracle gains\n**`begin_replacement_generation`** to drop trailing handoff-window\nconfirmed-input authorship, the game stub prunes state on snapshot load,\nand the drive loop defers confirmed-input sampling until the replacement\n**`LoadGameState`** completes. Runtime errors surface as\n**`hot_join_slot_unavailable`** (e.g. after **`GracefulRemove`**).\n> \n> Fleet tests cover clean-slot reactivation, fail-loud behavior on an\nalready-retired slot, and that pre-handoff **`StateDivergence`** is\nstill detected.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nd92c93d43164648341a058d5445c63f18e183d7a. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-07T15:52:16-07:00",
          "tree_id": "79f4fae49a524e705a2c4a3d9907c31028f75e80",
          "url": "https://github.com/wallstop/fortress-rollback/commit/bd69b090f97b0b582b2a36f9116f60ca8349d16f"
        },
        "date": 1783465030255,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 115,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 162,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 463,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 706,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1047,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 127859,
            "range": "± 1542",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 43852,
            "range": "± 3549",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1557,
            "range": "± 105",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "93542c2364e3a6d66550bd462769241fa1209293",
          "message": "Advance M3 simulation census coverage (#213)\n\n## Summary\n\n- Add a ReliableFifo simulation noise profile plus\n`LinkPolicy::retransmit_delay` so M3 can model reliable ordered\ntransports with head-of-line retransmission stalls instead of packet\nloss.\n- Rework the TCP-model fleet probes to use retransmit-delay windows and\nassert the premise with `retransmit_delayed > 0` and `dropped_by_policy\n== 0`.\n- Add the first `tests/simulation/census.rs` row for RTT far beyond\n`max_prediction`, asserting stall telemetry while the existing\nsimulation oracle proves liveness and state agreement.\n\n## Validation\n\n- `cargo nextest run --no-capture -E\n'test(retransmit_delay_delivers_would_drop_and_holds_later_sends) or\ntest(reliable_fifo_profile_is_lossless_ordered_and_storyline_free) or\ntest(link_policy_without_retransmit_delay_uses_zero_default) or\ntest(tcp_model_reliable_fifo_two_player_mesh_holds_invariants) or\ntest(tcp_model_reliable_fifo_four_player_mesh_holds_invariants) or\ntest(high_rtt_beyond_prediction_window_throttles_without_divergence)'`\n- `cargo fmt`\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- `cargo nextest run --test simulation --no-capture`\n- `cargo nextest run --no-capture` (2441 passed, 58 skipped)\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes are confined to test/simulation infrastructure (`SimNet`,\nschedules, fleet/census tests); production rollback protocol code is\nuntouched.\n> \n> **Overview**\n> Extends the simulation test stack so M3 can exercise **reliable\nordered transports** (TCP/WebRTC-style) instead of only UDP-like loss.\n> \n> **`SimNet`** gains `LinkPolicy::retransmit_delay`: when nonzero,\nburst/drop rolls **delay** would-be drops and block later sends on that\nlink until the retransmit deadline (head-of-line blocking), with a\n`retransmit_delayed` stat and `#[serde(default)]` so older schedule JSON\nstill deserializes as zero (UDP semantics).\n> \n> **Schedule harness** bumps schema to **9**, adds\n`BackgroundNoise::ReliableFifo` (lossless 30ms links, no random\nstorylines), and wires `retransmit_delay` through generated policies.\n> \n> **Fleet** TCP-model probes swap capture-and-hold `Hold` windows for\n**`SetLink`** policies with `drop_rate: 1.0` + 400ms retransmit delay,\nand assert `retransmit_delayed > 0` and `dropped_by_policy == 0`.\n> \n> **New `tests/simulation/census.rs`** row: two peers with ~240ms RTT\nand `max_prediction = 2` must pass the oracle, show **stall_count > 0**,\nand reproduce trace hash (prediction-window throttling without\ndivergence).\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n995beb63b3f3878cad424e1f87dfc2bcccfdc02f. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-08T11:40:38-07:00",
          "tree_id": "759ba1ecdfc0ec64b2f7eb13f4d4b7dbcd69bde6",
          "url": "https://github.com/wallstop/fortress-rollback/commit/93542c2364e3a6d66550bd462769241fa1209293"
        },
        "date": 1783536318405,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 113,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 162,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 459,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 707,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1035,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 128115,
            "range": "± 548",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 43637,
            "range": "± 248",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 66",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 83",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "ee8cb20a410f3a7403fba2dd11bda7b2a8efbc8a",
          "message": "Add frozen-queue network blip census row (#214)\n\n## Summary\n\nAdds the next M3 section 6.4 premise-asserted simulation census row for\na frozen dropped slot plus a sub-timeout survivor link blip.\n\n## What changed\n\n- Adds harness-level drained peer event counters, both aggregate and\nsplit by observing peer.\n- Adds payload-keyed peer event counters by observing peer so census\nrows can assert the exact endpoint named by drained events.\n- Adds `frozen_queue_survivors_resume_after_network_blip`, a hand-built\nschedule that:\n- gracefully removes peer 2 under `ContinueWithout`, freezing the\ndeparted slot;\n- blocks live survivor traffic between peers 0 and 1 for a sub-timeout\nwindow;\n- heals at the actual blip restoration step, so bounded recovery is\nanchored to the real link restoration;\n  - asserts blocked traffic was actually dropped by the fabric;\n- asserts both live survivors observed `PeerDropped` for removed peer 2;\n- asserts each survivor observed `NetworkInterrupted` and\n`NetworkResumed` for the other survivor's address;\n  - asserts bounded post-heal recovery ran and passed;\n  - asserts survivor confirmation progress and deterministic replay.\n- Extends the harness default-vs-explicit input regression to cover the\nnew event counter fields.\n\n## Validation\n\n- `cargo test --test simulation\nfrozen_queue_survivors_resume_after_network_blip -- --nocapture`\n- `cargo test --test simulation census -- --nocapture`\n- `cargo test --test simulation\ndefault_run_matches_explicit_stub_input_run -- --nocapture`\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- `git diff --check`\n- `npx markdownlint PLAN.md\nprogress/session-088-frozen-queue-network-blip-census.md --config\n.markdownlint.json --fix`\n- `cargo nextest run --no-capture` -> `2442 passed, 58 skipped`\n\n## Review follow-up\n\nCursor bugbot findings from the first revision are addressed in commit\n`7a7c5b6`: recovery is anchored at `blip_end`, event assertions are\npayload-specific, and `PeerDropped` propagation is asserted for survivor\n1. A second adversarial sub-agent review reported zero issues.\n\n## Notes\n\nPLAN.md and progress logs are updated locally per agent workflow, but\nthis repository ignores `PLAN.md` and `progress/**`. The H-RING\ncandidate row was also explored and produced a real red result; it\nshould be handled as a separate red-green investigation rather than\nincluded here.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes are confined to simulation tests and harness reporting;\nproduction rollback/session code is untouched.\n> \n> **Overview**\n> Extends the simulation **harness** so census rows can assert on\n**drained peer events**, not only end-state oracles. **`RunReport`** now\ncarries aggregate peer event counts, per-observer counts, and\n**payload-keyed** counts (`PeerEventKey` / `PeerEventPayload`) built\nwhile draining each peer’s event queue; **`peer_addr`** is exposed to\ntests for address-specific keys.\n> \n> Adds the M3 §6.4 census\n**`frozen_queue_survivors_resume_after_network_blip`**: a 3-peer\nschedule that gracefully removes peer 2 under **`ContinueWithout`**,\nblocks survivor traffic between peers 0 and 1 for a sub-timeout window,\nheals when links unblock, then asserts blocked drops, bounded post-heal\nrecovery, **`NetworkInterrupted`/`NetworkResumed`** per remote address,\n**`PeerDropped`** for the removed slot, survivor confirmation progress,\nand deterministic **`trace_hash`** replay.\n> \n> The default-vs-explicit stub input harness regression now also\ncompares the new event counter fields.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nfb198c31267102f08c07e10ca9d4258471c150dc. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-08T13:56:20-07:00",
          "tree_id": "bee5c93938c953f2f877122cb8b4ddc7a2157495",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ee8cb20a410f3a7403fba2dd11bda7b2a8efbc8a"
        },
        "date": 1783544455467,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 115,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 163,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 460,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 716,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1047,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 128279,
            "range": "± 2362",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 44197,
            "range": "± 455",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "824ea063b30b15672430f61521721bf448354ac8",
          "message": "Add sparse and multi-drop census rows (#215)\n\n## Summary\n\n- add a serializable simulation `SavePolicy` axis and wire it into P2P\nharness session builders\n- add harness observations for directed blocked drops and observed\n`LoadGameState` requests\n- add premise-asserted census rows for `SaveMode::Sparse` graceful-drop\nrollback and same-step multi-drop after asymmetric receipt loss\n\n## Validation\n\n- `cargo fmt --check`\n- `cargo test --test simulation\nsparse_save_mode_survives_graceful_drop_rollback -- --nocapture`\n- `cargo test --test simulation\nsame_step_multi_drop_after_asymmetric_block_converges -- --nocapture`\n- `cargo test --test simulation census -- --nocapture`\n- `cargo test --test simulation\nconfig_without_serde_default_fields_uses_defaults -- --nocapture`\n- `cargo test --test simulation\ndefault_run_matches_explicit_stub_input_run -- --nocapture`\n- `cargo test --test simulation census --features hot-join --\n--nocapture`\n- `npx markdownlint 'PLAN.md'\n'progress/session-089-m3-sparse-and-multidrop-census.md' --config\n.markdownlint.json --fix`\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- `cargo nextest run --no-capture` (2444 passed, 58 skipped)\n- `cargo nextest run --features hot-join --no-capture` (2687 passed, 58\nskipped)\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes are confined to simulation test infrastructure and census\ncoverage; production rollback code is only consumed via existing\n`SaveMode` APIs.\n> \n> **Overview**\n> Adds a serializable **`SavePolicy`** on simulation schedules (default\n**`EveryFrame`**) and wires it into P2P session builders via\n**`with_save_mode`**, so corpus runs can exercise **`SaveMode::Sparse`**\nwithout a schema bump.\n> \n> **SimNet** now tracks **per-directed-link blocked-drop counts**; the\nharness maps those to peer indices on\n**`RunReport::blocked_drops_by_link`** and records **`LoadGameState`**\nrequests as **`load_game_state_observations`**. Hot-join schedules with\nthree or more peers must use **`EveryFrame`** saving.\n> \n> Two **M3 §6.4 census** tests pin graceful-drop rollback under sparse\nsaves and same-step multi-drop after asymmetric blocks, with premise\nchecks (loads, rollbacks, **`PeerDropped`**, blocked links) plus the\nexisting oracle and trace-hash reproducibility.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n11c26963c9a6c29a5c92462192e1fbb9a98b918d. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-08T19:15:32-07:00",
          "tree_id": "1d12d311924178359d5d0b24650da601c039946b",
          "url": "https://github.com/wallstop/fortress-rollback/commit/824ea063b30b15672430f61521721bf448354ac8"
        },
        "date": 1783563606912,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 119,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 169,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 489,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 742,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1066,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 128746,
            "range": "± 618",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 43792,
            "range": "± 963",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "dfa4aa371d4c8a3232c1842cbc2d05c44c705c9c",
          "message": "Support Godot GDExtensions on wasm32-unknown-emscripten (#217)\n\n## Summary\n\n- keep browser-only JavaScript bridge dependencies out of the\n`wasm32-unknown-emscripten` graph while preserving browser\n`wasm32-unknown-unknown` support\n- move `ChaosSocket` to the same cross-platform monotonic clock as the\nprotocol and add exact browser/Emscripten compile, graph, and runtime\ngates\n- exercise Fortress inside real threaded and non-threaded Godot 4.6.3\nWeb GDExtension exports in Chromium\n- correct Matchbox, target-gating, clock, changelog, and migration\nguidance\n\n## Root cause\n\nFortress's quality-report timestamp previously called `js_sys::Date` for\nevery `wasm32` target. Godot loads Rust GDExtensions as Emscripten side\nmodules, where wasm-bindgen imports are unavailable. A second\nbrowser-only clock path remained in `ChaosSocket`, and the existing WASM\nCI covered only `wasm32-unknown-unknown`.\n\n## Validation\n\n- strict workspace Clippy\n- 2,473 nextest tests passed\n- browser and Emscripten five-feature compile matrices plus target\nClippy\n- Emscripten normal dependency graph contains no `wasm-bindgen*`,\n`js-sys`, or `web-sys`\n- browser ChaosSocket runtime smoke passed under\n`wasm-bindgen-test-runner 0.2.106`\n- Godot 4.6.3 + Emscripten 4.0.11 threaded/non-threaded exports passed\n2/2 Chromium tests\n- actionlint, workflow tests, cargo-deny, rustdoc, docs/wiki/link\nchecks, pre-commit, and agent preflight\n\nAddresses #216\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Breaking clock callback type on browser WASM and new timing paths\naffect networking telemetry; CI mitigates compile/graph/runtime risk but\nproduction Godot/browser combos still need careful integration testing.\n> \n> **Overview**\n> Enables **Godot Web GDExtensions** on `wasm32-unknown-emscripten` by\nrouting protocol quality-report RTT and all `ChaosSocket` timing through\n**`web_time::Instant`** (honoring `ProtocolConfig::clock`) instead of\nwall-clock/`js_sys`, and **breaking** `ChaosSocket::with_clock()` to\nreturn `web_time::Instant` on browser WASM.\n> \n> Adds **`scripts/ci/check-emscripten-dependencies.sh`** and expands\n**`wasm-check`** to a matrix (`wasm32-unknown-unknown` + Emscripten):\nfeature-matrix `cargo check`/`clippy`, Emscripten-only JS-bridge\nrejection, and a **`wasm-browser-smoke`** `wasm-bindgen-test` for\ndefault Chaos clock behavior. New **`godot-emscripten`** CI builds\nthreaded/non-threaded Godot 4.6.3 exports and runs Playwright probes\nagainst real protocol RTT smoke.\n> \n> Docs/README/migration now distinguish **browser vs Emscripten**\ntargets, Matchbox **0.14** adapter guidance, and monotonic-clock\nsemantics; workspace/docker wiring includes the new smoke crate.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n40a6cc376050e076288ce1d4fc9e9ca61cec3c93. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-10T10:18:39-07:00",
          "tree_id": "0fd8525db1385e93632b105fa4fb04df4c3e1694",
          "url": "https://github.com/wallstop/fortress-rollback/commit/dfa4aa371d4c8a3232c1842cbc2d05c44c705c9c"
        },
        "date": 1783704235970,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 115,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 161,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 430,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 682,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1005,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 125160,
            "range": "± 2809",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45761,
            "range": "± 274",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 101",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "abd2a7febab3e63358fb1324e20150be07a6e12c",
          "message": "Build simulation failure pipeline and harden disconnect recovery (#218)\n\n## Summary\n\n- build a stable, bounded simulation failure-artifact pipeline with\ndeterministic trace identities, replay, shrinking, corpus promotion, and\nstrict validation\n- expand lifecycle simulation coverage and add a release-mode nightly\nfleet spanning 8 shards, 1,000 disjoint seeds, N=2..16, 5,000 steps, and\nclean/mild/rough/reliable-FIFO networks\n- fix two production failures exposed by the new fleet: Halt\nconfirmation could rise after peer loss (D13), and stale delta-reference\nretransmissions could fail to re-ACK received history (D15)\n- pin the remaining lossy one-caller graceful-removal history rewrite as\nan explicit minimized known defect (D14) without weakening the oracle\n- extend the PeerDrop TLA+ model and update the deterministic sweep cost\nledger\n\n## Root causes\n\nD13 removed disconnected peers from the confirmation fold before\npreserving the last safe public confirmation bound. Later fold values\ncould therefore expose speculative fabricated-input frames as confirmed.\nThe session now latches and min-tightens a durable pre-mutation\nconfirmation ceiling across explicit, timeout, and propagated\nfail-closed paths.\n\nD15's missing-delta-reference input branch merged gossip but neither\napplied the packet's independent piggyback ACK nor re-emitted the\ncurrent cumulative ACK. One lost earlier ACK could leave an\nalready-received pending front forever and exhaust prediction. The\nbranch now applies valid piggyback ACK state and re-ACKs the receive\nhigh-water.\n\n## Test machinery\n\nFailure artifacts use a stable schema, bounded diagnostic payloads, full\nschedules, exact replay options, atomic publication, and a stable\nfinal-step trace. The shrinker preserves failure classes,\ndouble-confirms candidates, catches candidate panics, remaps peers, and\nuses bounded ddmin plus event-adjacent/geometric schedule checkpoints.\nCorpus promotion validates and reproduces through Rust before a locked\nno-clobber publish.\n\nSerialized schedules are bounded before execution: 2..=100,000 steps,\nprediction <=127, 1..=1,000 ms step duration, <=60 s link-delay fields,\n<=100,000 events, and <=8 MiB corpus JSON.\n\n## Validation\n\n- `cargo fmt --all -- --check`\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `cargo nextest run --no-capture` — 2,494 passed, 69 skipped\n- ignored D14 exact fixture — frame-327 ConfirmedInputDivergence\nreproduced repeatedly\n- full nightly shard replay — 125/125 seeds at 5,000 steps in release\nmode\n- `scripts/verification/verify-tla.sh --quick PeerDrop` — 4,372\ngenerated / 1,190 distinct states\n- `cargo doc --no-deps`\n- `actionlint`, Markdown lint, shell syntax, and repository agent\npreflight\n- repeated adversarial review — zero remaining issues, including minor\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **High Risk**\n> Changes core P2P session disconnect/Halt semantics, input-protocol ACK\nbehavior, and hot-join recovery paths—areas where subtle regressions\ncause desync or permanent stalls.\n> \n> **Overview**\n> Adds a **nightly deterministic simulation fleet** (8 shards × 125\nrelease-mode 5,000-step seeds), a **failure-artifact → corpus\npromotion** pipeline, and fixes two rollback/network bugs found by that\ncoverage.\n> \n> **`DisconnectBehavior::Halt`** now latches a durable\n**`halt_confirmed_ceiling`** at the pre-disconnect safe bound so\n`confirmed_frame()` cannot rise into speculative default-input territory\nafter drops; fail-closed paths capture the ceiling before mutation,\n**`check_initial_sync`** and hot-join snapshot apply no longer resurrect\na halted session, and **PeerDrop.tla** models the capped confirmation\nfold.\n> \n> **Stale input retransmissions** whose delta reference was pruned still\nmerge gossip but previously skipped ACK handling; the protocol now\n**applies piggyback ACKs** and **re-emits cumulative `InputAck`** at the\nreceive high-water so a lost earlier ACK cannot strand `pending_output`\nand deadlock prediction windows.\n> \n> Simulation harness gains **bounded failure artifacts**, **corpus\nreplay** (including a pinned cold-start gossip stall schedule), nightly\nlifecycle matrices with retirement quarantined under lossy noise, and an\nexplicit **ignored known defect** for lossy graceful removal rewriting\nconfirmed history; D13 partition-under-Halt tests flip from red to\ngreen.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n382f8fc568020eeb469dd2f15311d4ba68234b16. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\n---------\n\nCo-authored-by: copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>",
          "timestamp": "2026-07-10T16:29:31-07:00",
          "tree_id": "e8a8d876fe74696427a1ccd860f97391cc6ae31c",
          "url": "https://github.com/wallstop/fortress-rollback/commit/abd2a7febab3e63358fb1324e20150be07a6e12c"
        },
        "date": 1783726448248,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 112,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 160,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 434,
            "range": "± 17",
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
            "value": 1025,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 136213,
            "range": "± 524",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45259,
            "range": "± 1258",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "59f0dcd94d91462f299d6eece700ed1a663186c8",
          "message": "Harden hot-join metrics and bounded event retention (#219)\n\n## Summary\n\n- Pin the public N-peer hot-join metrics lifecycle: incomplete →\ncompleted, multi-poll, positive injected-clock latency, and stable\ncompletion.\n- Close D9 across P2P, spectator, and replay sessions with fallibly\nreserved event queues and allocation-free priority retention at\ncapacity.\n- Retain routine-vs-durable ordering honestly: evict queued routine\nevents first; reject an incoming routine against a durable-only queue;\notherwise replace the oldest durable event and record the exact loss.\n- Add replay metrics/observer integration, constructor\nallocation-failure coverage, and explicit disconnect-emission tracking\nfor saturated queues.\n- Update user-facing retention contracts and changelog.\n\n## Validation\n\n- `cargo fmt --all -- --check`\n- strict workspace clippy with `tokio,json,hot-join`\n- rustdoc with `tokio,json,hot-join`\n- changed-file agent preflight, changelog rule, link checks, and\nmarkdown lint\n- default nextest: 2,506 passed, 69 skipped\n- hot-join nextest: 2,753 passed, 69 skipped\n- nine adversarial sub-agent rounds; final verdict: zero D9 issues\n- mutation check proves the N-peer metrics assertion detects removal of\nactivation recording\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Changes how session event queues drop notifications under load\n(including disconnect/desync paths) across P2P, spectator, and replay;\nbehavior is heavily tested but affects core session lifecycle semantics.\n> \n> **Overview**\n> Replaces **FIFO event-queue trimming** with **routine-vs-durable\nretention** shared by P2P, spectator, and replay sessions. At capacity,\nthe oldest queued routine/advisory event is evicted first; if the queue\nis durable-only, incoming routine events are dropped and incoming\ndurable events replace the oldest durable slot. Enqueues go through\n`enqueue_event_bounded` with **fallible queue reservation** at session\nbuild time, **inline** `SessionMetrics` discard accounting, and\nrate-limited overflow warnings on every emission path (not only after\n`handle_event`).\n> \n> **Replay** gains the same bound plus `ReplaySession::metrics()`,\nbuilder wiring for queue size and violation observer, and\nallocation-failure handling. **P2P disconnect** uses tracked emission so\na saturated queue does not duplicate `Disconnected` when graceful-drop\ncleanup fails after the terminal event was already enqueued.\nDocs/changelog describe the retention contract; regressions cover\ndurable canaries, desync under pressure, and replay validation overflow.\n> \n> N-peer **hot-join metrics** tests assert completion, multi-poll span,\nand positive clock latency on the public mesh path.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nda603e5b03be2905228308c87aec627b792969f7. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-10T19:31:20-07:00",
          "tree_id": "9088d71ea74a20f2948035f25a7262dadca9526e",
          "url": "https://github.com/wallstop/fortress-rollback/commit/59f0dcd94d91462f299d6eece700ed1a663186c8"
        },
        "date": 1783737390585,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 122,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 163,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 468,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 718,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1051,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 125814,
            "range": "± 1543",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 47323,
            "range": "± 778",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1406,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1605,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "2b2ba0161abf4af5011484ff148eece8a611e137",
          "message": "Add asymmetric minority partition census (#220)\n\n## Summary\n\n- add a premise-asserted N=5 one-way/minority partition census for\nPLAN.md §28.3(a,b)\n- pin the resulting D14 confirmed-history rewrite as an ignored\nknown-red regression until the M5 coordinated drop barrier lands\n- document the devcontainer's connector-first GitHub workflow: local Git\nover the VS Code-forwarded SSH agent, connected GitHub app for PR/review\noperations, and `gh` only when an applicable workflow requires it\n\n## Why\n\nThe asymmetric/minority partition row had remained unexecuted. The\nexperiment showed that a single blocked `4→0` direction forms a\nfour-peer majority and one-peer island through disconnect gossip, but\nalso independently reproduces D14. The regression now proves the exact\ncausal premises and accepts only the D14-shaped peer-4 input rewrite.\n\nPrevious automation also incorrectly treated an unauthenticated `gh` CLI\nas a publish blocker even though the devcontainer's SSH Git transport\nand connected GitHub app were both functional. The canonical LLM\ninstructions now describe the verified hybrid path.\n\n## Validation\n\n- `cargo fmt --check`\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `cargo nextest run --no-capture` (2,506 passed)\n- focused census suite under default and `hot-join`\n- ignored D14 census regression executed directly\n- `scripts/docs/check-llm-skills.sh`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- `git push --dry-run origin HEAD:refs/heads/dev/wallstop/hardening-30`\n\nNo changelog entry: production behavior and public API are unchanged.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes are confined to simulation tests and agent documentation;\nproduction networking behavior and public APIs are unchanged.\n> \n> **Overview**\n> Adds a **PLAN §28.3** simulation census for a five-peer mesh with a\n**one-way block on `4→0`**: peer 0 times out and drops peer 4 while\n`0→4` stays open, forming a four-peer majority and a one-peer island\nunder `ContinueWithout`.\n> \n> The new\n**`one_way_minority_partition_rewrites_confirmed_history_known_defect`**\ntest is **`#[ignore]`** and pins the **D14** failure\nshape—`ConfirmedInputDivergence` rewriting only peer 4’s input on the\nmajority, plus expected gossip/drop/network-interruption premises and\ndeterministic trace replay—until an M5 coordinated drop barrier is\nexpected to fix it.\n> \n> **`.llm/context.md`** tightens the quality checklist bullets and adds\n**GitHub Access in Devcontainers**: prefer `git ls-remote` / `git push\n--dry-run` and the connected GitHub app over treating unauthenticated\n`gh` as a publish blocker.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n73e139e46d1821d77aff6a1a0992c46f4581629d. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-10T22:03:24-07:00",
          "tree_id": "f5867372dbfa183ec5cee17a89836d71fa8cab36",
          "url": "https://github.com/wallstop/fortress-rollback/commit/2b2ba0161abf4af5011484ff148eece8a611e137"
        },
        "date": 1783746474325,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 114,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 160,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 502,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 692,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1024,
            "range": "± 104",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 139900,
            "range": "± 351",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45274,
            "range": "± 256",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 84",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "1e3056cf433220ae19a20ce7a2b177db57d0b011",
          "message": "Expose unknown-source packet telemetry (#221)\n\n## What changed\n\n- add `SessionMetrics::unknown_source_packets` for decoded messages from\nunregistered endpoints\n- instrument both P2P and spectator receive paths\n- emit one lifetime-bounded warning with the first offending address\n- document the decode boundary and serialize the new counter through the\nJSON metrics API\n\n## Why\n\nUnknown-source traffic was silently discarded, making NAT rebinding,\nstale traffic, and spoofing indistinguishable from pure peer silence.\nThis closes PLAN.md §30.3a and provides the observable prerequisite for\nthe planned `Rebind{peer}` simulation lifecycle.\n\n## Validation\n\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- `cargo fmt --check`\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `cargo nextest run --no-capture` — 2,509 passed, 70 skipped\n- `cargo doc --no-deps`\n- adversarial review: zero remaining issues\n\n## Review Readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS\n- Determinism: PASS\n- Agent preflight: PASS\n- Error handling: PASS\n- Tests breadth: PASS\n- Design log reviewed: N/A\n- CHANGELOG reviewed: YES\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Changes touch rollback input retention, session liveness on queue\nexhaustion, and large simulation/network test paths; unknown-source\ncounting alone is low risk but bundled sync-layer and fail-closed\nreceive handling can affect live P2P stability.\n> \n> **Overview**\n> Adds **`SessionMetrics::unknown_source_packets`** and wires P2P and\nspectator receive loops to count decoded messages from addresses that\nare not configured remotes/spectators (or hosts), emit **one\nper-session** `NetworkProtocol` warning naming the first offender, and\nexpose the counter in JSON metrics. Malformed pre-decode datagrams stay\noutside this boundary.\n> \n> Separately hardens **input recovery at the redundancy limit**: full\nrings can **reclaim** only history at or below a **global\nrollback-window floor** from `SyncLayer`, keeping the floor frame in a\n**`reclaimed_floor_input`** side slot; unsafe overlap fails without\nmutating receipt state. **Rollback/synctest construction** and\n**`set_input_delay`** now require `max_prediction + input_delay <\nqueue_length`; remote inputs that cannot be retained trigger\n**fail-closed** disconnect instead of advancing ACK state.\n> \n> **Test/simulation fabric** grows substantially: `SimNet` gains\noptional **Gilbert–Elliott** loss, **IPv4-style fragmentation** loss\n(size-aware), **NAT rebind** on live sockets, and richer link telemetry;\nsimulation **census** and **baseline_sweep** add schedules for those\nfaults, input-window boundaries, and hot-join scenarios. Docs\n(**CHANGELOG**, **migration.md**, design-history) and an unstable\n**`message_metadata`** test hook accompany the behavior changes.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n810ca0e01611b7ce16e7cb6888bf69987a51d42d. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-11T08:29:49-07:00",
          "tree_id": "4a148d8682768320805c74bf676c293b9f08042c",
          "url": "https://github.com/wallstop/fortress-rollback/commit/1e3056cf433220ae19a20ce7a2b177db57d0b011"
        },
        "date": 1783784047479,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 98,
            "range": "± 4",
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
            "value": 371,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 593,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 857,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 99032,
            "range": "± 321",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 38108,
            "range": "± 1350",
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
            "value": 1242,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "69f5c51cf3ab7e5559ac91fbaaed31df809f7a46",
          "message": "Model bounded bandwidth queueing (#222)\n\n## What changed\n\n- add schema-v14 deterministic per-directed-link token-bucket bandwidth\nand bounded tail-drop queueing to `SimNet`\n- bound queued metadata by bytes, per-link reservations, and\nwhole-fabric reservations\n- preserve backlog ordering across policy replacement, `HealAll`, and\nNAT rebind\n- add global/directional trace telemetry, schema validation, serde\ncoverage, and shrinker support\n- add a matched two-peer census row proving queue growth, bounded tail\ndrop, asymmetric wait recommendations, deterministic replay, and bounded\nrecovery\n\n## Why\n\nThe hardening plan could model delay, loss, fragmentation, and reliable\nHOL stalls, but not constrained uplinks or bufferbloat. That left\nH-BLOAT and the bandwidth/fragmentation interaction structurally\nuntestable.\n\nThis PR establishes the bounded deterministic primitive and first\npremise-asserted data without claiming the still-open H-BLOAT\nfeedback-loop verdict.\n\n## Validation\n\n- `cargo nextest run --features tokio,json,hot-join --no-capture` —\n3,003 passed, 71 intentionally skipped\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- two independent adversarial review loops, both final zero-issue\nverdicts\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Large new shaper state machine in shared test infrastructure affects\ndeterministic replay and trace hashes, but changes stay in\nsimulation/test code rather than production rollback paths.\n> \n> **Overview**\n> Adds optional **token-bucket bandwidth** with **bounded tail-drop\nqueueing** on directed links in the test `SimNet`, bumping simulation\nschedules to **schema v14**.\n> \n> `LinkPolicy` gains `BandwidthPolicy` (rate, burst, queue capacity)\nwith caps on burst/queue bytes and reservation counts. Sends run through\nuplink shaping **before** loss, fragmentation, and duplication; delivery\ntime is `shaped_departure + delay`. Oversize payloads, full queues,\nreservation caps, and unrepresentable deadlines fail closed with\ndedicated stats. **Policy swap**, **`HealAll`**, and **NAT rebind** keep\nor move backlog so new traffic cannot jump ahead of admitted shaping.\n> \n> Telemetry is wired through `SimLinkStats`, `SimNetStats`, harness\n`TraceNetStats`, schedule validation, and the failure shrinker\n(bandwidth stripped as its own axis). Extensive unit tests cover queue\ndelay, fractional refill, horizon drain after disabling shaping, and\nrebind behavior; a **census** row asserts queue growth, tail drops,\nasymmetric wait recommendations, replay identity, and recovery on a\nconstrained 0→1 link.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n6015b9568508007f9bf61880b5ca6856bcc9f386. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-11T11:53:53-07:00",
          "tree_id": "3a554db434118dbaed75f2c118c821c4d6122003",
          "url": "https://github.com/wallstop/fortress-rollback/commit/69f5c51cf3ab7e5559ac91fbaaed31df809f7a46"
        },
        "date": 1783796317146,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 113,
            "range": "± 2",
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
            "value": 471,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 754,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1089,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 139306,
            "range": "± 5439",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 44766,
            "range": "± 383",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 86",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "c371e28b33284f74aedd450e9a3f5e7946283d9d",
          "message": "Model clock-skewed frame production (#223)\n\n## Summary\n\n- add schema-v15 deterministic 60 Hz frame gating driven by each peer's\nskewed clock\n- execute and nightly-gate a matched one-hour H-SKEW experiment with\nbounded lag, correction duty, and work-amplification evidence\n- preserve schema <=14 lockstep behavior while bounding gated schedules\nto one opportunity per peer per outer step\n- carry progress/cost evidence through trace identity, failure\nartifacts, replay validation, and shrinking\n\n## Findings\n\nAt +0.1% skew, the fast peer receives exactly 216 extra opportunities\nper virtual hour and obeys 213 correction frames with zero stalls.\nConfirmation lag stays flat at 4–5 frames. The experiment also exposes a\nstill-open cost issue: aggregate simulation work rises 13.3% and\nresimulation rises 28.3% versus the exact-clock control. Nightly\nceilings prevent this from worsening while W-TIME follow-up investigates\nit.\n\n## Validation\n\n- `cargo nextest run --features tokio,json --no-capture` — 2,760 passed,\n72 skipped\n- `cargo nextest run --test simulation --no-capture` — 287 passed, 25\nskipped\n- release hour-equivalent H-SKEW control + skew + replay — passed\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings` — passed\n- `actionlint .github/workflows/ci-simulation-nightly.yml` — passed\n- agent preflight — passed earlier; final rerun reached a pre-existing\nlong-running doc-claims sort and was stopped after all preceding checks\npassed\n- local adversarial review loop — final verdict: zero issues\n- Cursor round 1 finding fixed in `e74baa2`; exact-head re-review\nrequested\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Changes the deterministic harness frame loop and trace/artifact\nidentity (schema bumps), but production rollback paths are untouched and\nlegacy lockstep schedules stay default-compatible.\n> \n> **Overview**\n> Adds **schema v15** simulation support so each peer can accrue **60 Hz\nframe opportunities from its own skewed clock**\n(`FrameModel::SkewGated60Hz`), while **lockstep remains the default**\nfor older schedules. The harness now tracks **frame opportunities**,\n**obeyed wait frames**, and up to **12 progress samples**, folds them\ninto trace identity for schema ≥15, and bumps **failure artifacts to\nschema v3** with the same fields plus stricter replay checks.\n> \n> **H-SKEW** is exercised with new fleet tests: a short deterministic\nskew-gated probe and an **ignored hour-equivalent** run (+0.1% → 216\nextra opportunities/hour) that asserts bounded lag, correction duty, and\nwork/resimulation ceilings. **Nightly CI** now runs that experiment\nalongside the sharded fleet. Schedule limits rise (**250k steps**,\nskew-gated cadence validation); shrink can collapse `SkewGated60Hz` when\nunnecessary.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\ne2c53ff7fd531a506a708c983e8e47f8a8f97f38. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-11T15:16:33-07:00",
          "tree_id": "0204b4d3b29b088d3234092e2b82ddab6ae6879e",
          "url": "https://github.com/wallstop/fortress-rollback/commit/c371e28b33284f74aedd450e9a3f5e7946283d9d"
        },
        "date": 1783808466348,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 114,
            "range": "± 3",
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
            "value": 446,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 716,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1035,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 133172,
            "range": "± 2112",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46207,
            "range": "± 835",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 94",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "5ebb23a7b97c142e8fc89281af56f87d58f4ee8e",
          "message": "Add bounded control-loop simulation evidence (#224)\n\n## What\n\n- quantify H-SKEW rollback cost across mirrored peer orientation and\nfour scheduler cadences\n- add schema-v16 bounded endpoint RTT/advantage/backlog and live\nbandwidth queue/drain samples\n- preserve schema-v15 serialized trace identity and artifact replay\ncompatibility\n- execute matched H-OSC, H-BLOAT, and H-ASYM experiments with direct\nevidence\n\n## Why\n\nSeveral open PLAN hypotheses could only be discussed through aggregate\ncounters. This adds at most 12 deterministic directed-link samples per\nrun, allowing matched causal experiments without unbounded traces.\n\nThe resulting evidence:\n\n- perfectly symmetric 100 ms H-OSC controls are inert (zero\nrecommendations/skips), so perturbation-driven A10 remains\n- the N=2/4-byte H-BLOAT row shows identical bounded queue\nsamples/cumulative maxima while obedience cuts work 42.7%; scale and\nbetween-cut behavior remain open\n- 10/200 ms H-ASYM confirms a seven-frame throughput and 18-vs-11 stall\nsplit, but falsifies the predicted wait-recommendation mechanism at this\nbound\n\n## Impact\n\nTest/simulation infrastructure only. No production API, wire format,\nchangelog-relevant behavior, or runtime behavior changes.\n\n## Validation\n\n- agent preflight: PASS\n- strict workspace clippy with tokio,json: PASS\n- full nextest: 2,743 passed, 73 skipped\n- focused schema compatibility, artifact validation, H-OSC, H-BLOAT,\nH-ASYM, SimNet drain tests: PASS\n- ignored release H-SKEW hour and cadence/orientation probes: PASS\n- adversarial sub-agent review: zero issues\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes are confined to test/simulation harness and SimNet telemetry;\nno production runtime or wire-format behavior is modified.\n> \n> **Overview**\n> Adds **schema v16** simulation harness support: up to ~12\ndeterministic `ProgressSample` rows per run now optionally carry\nper-directed **endpoint** gauges (RTT, remote frame advantage, pending\noutput) and **live bandwidth queue** snapshots (queued bytes/datagrams,\ndrain delay). **Schema 15** keeps empty gauge vectors and unchanged\nserialized trace shape; artifact validation enforces directed-link\nordering and counts only for v16+.\n> \n> `SimNet` exposes read-only `SimBandwidthState` via\n`bandwidth_states()` without advancing virtual time or mutating queues.\n> \n> Matched census/fleet tests use the new series for causal evidence:\n**H-BLOAT** (obey vs ignore wait recommendations with identical queue\nsamples), **H-OSC** (symmetric delay does not trigger mutual sleep),\n**H-ASYM** (throughput bias without wait recommendations), tighter\n**bandwidth-queue** census assertions, and additional **H-SKEW** cost\nobservability (lower-bound assertions on hour test; ignored cadence\nmatrix probe).\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\ndf35e189e982aa28d0981e917519afa6f9ace358. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-11T16:41:52-07:00",
          "tree_id": "d96e8274539cb56e6e445fbf9719eaf48ce82888",
          "url": "https://github.com/wallstop/fortress-rollback/commit/5ebb23a7b97c142e8fc89281af56f87d58f4ee8e"
        },
        "date": 1783813588865,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 132,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 181,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 448,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 705,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1016,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 132960,
            "range": "± 3338",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46217,
            "range": "± 280",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 102",
            "unit": "ns/iter"
          },
          {
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "0b70f028557f9df15a57565345b0aa5773b8489d",
          "message": "Add protocol v1 verification and scale hardening evidence (#225)\n\n## Summary\n\n- add a TLC-enforced protocol-v1 handshake model covering safety, fair\nmatching, mismatch rejection, and a required non-vacuity counterexample\n- complete the missing `P2PSession::metrics` and `Message::encoded_len`\nCriterion baselines\n- add data-backed H-BLOAT, H-META-RB, and H-POLLCAP simulation rows,\nexact replay evidence, receive-cap diagnostics, and nightly coverage\n- document the preferred VS Code Git/GitHub integration order for agent\nworkflows\n\n## Why\n\nThe hardening plan requires measurement and formal entrance gates before\nthe protocol-v1 wire break. These changes close several open evidence\ngaps without changing production protocol behavior:\n\n- protocol-v1 handshake assumptions are model-checked before\nimplementation\n- N=16 bandwidth/fragmentation behavior is measured rather than inferred\n- rollback-storm persistence is bounded under fixed cadence without\noverstating CPU-feedback coverage\n- the per-socket 256-message cap is shown to defer, not drop or starve,\na planted 270-message typed receive storm\n\n## Results\n\n- H-BLOAT N=16/32-byte treatment reduces tail drops and maximum queue\ndelay, while exposing higher simulation work/resimulation cost\n- H-META-RB returns to the matched fixed-cadence control within the\nrecovery window\n- H-POLLCAP returns 256 messages, retains 14, drains subsequently, and\nadds at most one synchronization step versus cap 512\n- benchmark upper bounds: `P2PSession::metrics` 11.255 ns;\n`Message::encoded_len` 0.845 ns\n\n## Validation\n\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`\n- `cargo nextest run --no-capture`: 2,760 passed\n- `cargo nextest run --features hot-join --no-capture`: 3,012 passed\n- `bash scripts/verification/verify-tla.sh`: 19/19 gates passed,\nincluding the expected counterexample\n- `python3 scripts/ci/agent-preflight.py --all --auto-fix`\n- focused nightly H-BLOAT and H-POLLCAP release rows and H-META-RB\nregression\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes are confined to specs, verification scripts, benchmarks, test\nharness, and agent docs; production `src/` protocol behavior is not\nmodified in this diff.\n> \n> **Overview**\n> Adds **formal and simulation evidence** ahead of protocol-v1 wire\nwork, without changing production session/protocol behavior.\n> \n> **TLA+:** New `SyncHandshakeV1` module with safety (arbitrary loss),\nfair matching liveness, mismatch liveness, and an CI-enforced\n**handlers-disabled** mutation that must violate `EventuallyBothSynced`.\n`verify-tla.sh` registers the new specs, runs TLC from `specs/tla`, and\ntreats the mutation as a distinct expected-failure gate (not a generic\nTLC error).\n> \n> **Benchmarks:** Criterion coverage for allocation-free\n`P2PSession::metrics()` and `Message` wire-length via\n`__internal::message_metadata`.\n> \n> **Simulation harness:** `SimNet` gains configurable per-poll receive\nlimits and `SimReceiveStats`; `RunOptions::receive_message_limit` drives\ncaps and records **first Running / first Synchronized** timing. New\nignored-nightly probes: **H-POLLCAP** (270-message storm defers at 256,\ndrains without starvation), **H-BLOAT** (N=16 bandwidth/fragmentation vs\n`AppModel::Obey`), and **H-META-RB-OPENLOOP** (rollback work after RTT\nspike decays under fixed cadence). Nightly workflow filter includes the\ncensus H-* tests.\n> \n> **Docs:** `AGENTS.md` / `.llm/context.md` prefer callable VS Code Git\nand GitHub PR extensions over `gh` when available.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\ndc2993b63564e14e907d444b7373a5027ec903d4. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-11T21:03:28-07:00",
          "tree_id": "871f2705757b883c52f7554eb0e23ed79a5f4d93",
          "url": "https://github.com/wallstop/fortress-rollback/commit/0b70f028557f9df15a57565345b0aa5773b8489d"
        },
        "date": 1783829306440,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 115,
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
            "value": 444,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 705,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1023,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 124640,
            "range": "± 404",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46162,
            "range": "± 857",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 103",
            "unit": "ns/iter"
          },
          {
            "name": "P2PSession/metrics",
            "value": 16,
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
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "b535d3b53fa2b7c44a5074565321aced172bfd75",
          "message": "Land protocol v1 and coordinated graceful drop (#226)\n\n## Summary\n\n- retain the session-109 20-second default sync timeout and negative\nwire-frame validation\n- add the protocol-v1 12-byte prelude, feature-independent tags,\nvalidated 32-bit connection IDs, and tag-17 explicit-disconnect\n`Goodbye`\n- add the fixed-width v1 session-config handshake and canonical digest\n- fail closed with durable `IncompatibleSession` events on every\ndeterministic configuration mismatch\n- add bounded `codec::{encode_framed, FrameDecoder}` helpers for TCP and\nother raw byte streams\n- add inclusive 1,200-byte portability and 1,472-byte IPv4/UDP\nfragmentation diagnostics with distinct saturating `PeerMetrics`\ncounters\n- add tags 18–22 and a coordinated\nprepare/inventory/backfill/ready/commit graceful-drop barrier that never\nretracts exposed confirmed history\n- add bounded retained-history backfill, atomic freeze-at-cut,\ngeneration fencing, fail-closed timeout handling, and deterministic\nconcurrent-drop rebasing\n- add the `CoordinatedPeerDrop` TLA+ family with fair/fault models and\nan expected ImmediateMin counterexample\n- document best-effort local send/drop semantics, TCP latency/HOL\nbehavior, WebRTC/QUIC/WebTransport guidance, and the transport security\nboundary\n- correct built-in UDP/Tokio oversized-send warnings to report required\nencoded bytes rather than buffer capacity\n\n## Compatibility\n\nThis is the coordinated 0.10.0 breaking wire/API transition. Protocol v1\nrejects legacy 0.9 packets in both directions, and exhaustive matches\nmust handle the new `FortressEvent::IncompatibleSession` and\n`EventKind::IncompatibleSession` variants. Existing event discriminants\nare preserved by appending the new variant. The stream length prefix is\ntransport-local and does not change datagram bytes or deterministic\nstate. `PeerMetrics` is non-exhaustive, so its new counters are\nadditive.\n\n## Validation\n\n- default nextest: 2,827 passed, 73 skipped\n- hot-join nextest: 3,082 passed, 73 skipped\n- default and hot-join/tokio/json all-target clippy with `-D warnings`\n- focused framing split/concatenation/poison/property tests and exact\n1,199/1,200/1,471/1,472 boundary tests\n- 35-test peer-drop slice, D14 lossy/minority-partition simulations, and\nall five new tag byte goldens\n- coordinated-drop TLC suite: base, fair, faults, and expected\nImmediateMin counterexample\n- sweep PR gate\n- semver checks against 0.9.0\n- rustdoc, 223 doc tests (169 passed, 54 ignored), and examples\n- agent preflight, links, semantic doc claims, markdownlint, version\nsync, and changelog policy\n- two-pass adversarial review: all five first-pass findings fixed by a\nseparate pass; fresh re-review reported zero issues\n- Miri budget follow-up: expensive new property probes remain in\nordinary nextest but are skipped under Miri; deterministic framing\ncoverage remains on every Miri shard\n\nExact head: `86e8cd67aa3c7f7ede6c6e401c518826fe875daf`.\n\nRemaining planned wire work: complete the immutable protocol-v1/legacy\ngolden suite beyond the five pinned D14 variants.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **High Risk**\n> Breaking wire protocol and session handshake require fleet-wide\nupgrades; coordinated graceful drop and incompatible-session handling\nchange core disconnect and sync failure semantics.\n> \n> **Overview**\n> **0.10.0** is a coordinated breaking release: all peers must upgrade\ntogether for **protocol-v1** on the wire and for new session/API\nbehavior.\n> \n> **Wire and sync:** Packets gain a versioned prelude (`F5 52`, version\n1, flags, 32-bit connection ID) and reject legacy 0.9 traffic. Sync\nrequest/reply now carry a fixed session-config block and canonical\ndigest; mismatches emit a sticky\n**`FortressEvent::IncompatibleSession`** (new exhaustive match arms on\n`FortressEvent` / `EventKind`). Default **`SyncConfig::sync_timeout`**\nbecomes **20s** (`None` restores unlimited retries). Tag 17\n**`Goodbye`** replaces the old input disconnect byte; graceful-drop tags\n**18–22** support a multi-phase barrier. Datagram bytes are unchanged\nfor UDP; **`codec::encode_framed`** and **`FrameDecoder`** add a\ntransport-local u32-LE envelope for TCP/raw streams.\n> \n> **Graceful drop:** **`remove_player`** and **`ContinueWithout`**\nauto-removal no longer freeze locally on gossip; survivors run a\n**prepare → inventory → backfill → ready → commit** certificate, hold\nconfirmation until a non-retracting cut, then emit **`PeerDropped`**\nonly on commit (failure returns to **`Synchronizing`** without the\nevent). **`CoordinatedPeerDrop`** TLA+ models this with fair/fault\ncompanions and an **ImmediateMin** mutation counterexample.\n> \n> **Observability and docs:** **`PeerMetrics`** adds saturating\n**1,200-byte** portability and **1,472-byte** fragmentation-risk\ncounters (diagnostics only). New **threat model**, migration\n**0.9→0.10** wire section, and transport guidance (best-effort send, TCP\nHOL, prefer datagram rollback paths).\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n86e8cd67aa3c7f7ede6c6e401c518826fe875daf. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-12T09:31:44-07:00",
          "tree_id": "77898bbfd2001bdc6f458cb794764a7b6aa3566d",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b535d3b53fa2b7c44a5074565321aced172bfd75"
        },
        "date": 1783874236438,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 115,
            "range": "± 1",
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
            "value": 469,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 700,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1024,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 129206,
            "range": "± 1595",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46933,
            "range": "± 119",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1557,
            "range": "± 100",
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
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "991559cca913b7f7b0495d444e1224f28bdca423",
          "message": "Complete protocol v1 wire compatibility gates (#229)\n\n## Summary\n\n- add immutable exact-byte fixtures for every protocol-v1 message tag\nand recorded v0.9 compatibility fixtures\n- prove legacy rejection in both directions, including a real UDP\nlegacy-handshake timeout regression\n- enforce released wire-golden immutability in worktree, staged, and\ncommitted PR diffs, including version-successor registration and\nadversarial bypass coverage\n- wire the policy into pre-commit, agent preflight, and PR CI\n\n## Why\n\nProtocol v1 was implemented through the final coordinated-drop tags, but\nM5 still lacked its complete immutable golden suite and enforcement.\nWithout a committed-diff gate, historical fixtures could also be\nrewritten in a clean PR checkout without a protocol-version increase.\n\n## Impact\n\nThis is test and CI hardening only; it does not change production wire\nbytes. Future wire changes must increase `PROTOCOL_VERSION` and add a\nload-bearing successor golden suite.\n\n## Validation\n\n- `cargo nextest run --no-capture`: 2,833 passed, 73 skipped\n- `cargo nextest run --features hot-join --no-capture`: 3,087 passed, 73\nskipped\n- `cargo clippy --workspace --all-targets --features\ntokio,json,hot-join`\n- `cargo doc --no-deps`\n- `python3 -m pytest scripts/tests --no-header -q`: 1,627 passed\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n- `actionlint .github/workflows/ci-version-sync.yml`\n- repeated adversarial review passes; final pass reported zero issues\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Test and CI/policy hardening only; production wire encoding is\nunchanged, though the immutability hook is security-relevant for\nprotocol compatibility.\n> \n> **Overview**\n> Adds **immutable protocol-v1 exact-byte fixtures** for every message\nvariant, **recorded v0.9 legacy packets**, and a shared\n`assert_wire_golden_suite` harness in `codec.rs`, plus tests that legacy\nbytes are classified/rejected and that a real UDP legacy handshake times\nout without synchronizing.\n> \n> Introduces **`check-wire-golden-immutable.py`**, which blocks edits to\nreleased `wire_golden_*` files unless `PROTOCOL_VERSION` increases and a\nproperly wired successor suite is the sole active registration in\n`codec.rs` (with Rust-aware checks to resist comment/cfg/macro\nbypasses). The hook runs on **every pre-commit** (`--cached`), in\n**agent preflight** (`--local`), and in **PR CI** (`--base-ref` against\nthe merge base with full git history).\n> \n> Wires the hook into workflow path filters and adds broad **pytest**\ncoverage for the checker and hook policy.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nfcb91710e27d46e973b461755408093e31a1c3ea. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-12T15:06:23-07:00",
          "tree_id": "7d047c618f00648b25cef4df2c8e0cd8e6d3167a",
          "url": "https://github.com/wallstop/fortress-rollback/commit/991559cca913b7f7b0495d444e1224f28bdca423"
        },
        "date": 1783894283423,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 114,
            "range": "± 2",
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
            "value": 438,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 700,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1018,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 140048,
            "range": "± 374",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46758,
            "range": "± 873",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 85",
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
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "bb7122b59d9311090065c5bb4ea266bb0bc001fd",
          "message": "Publish M6 operations guides and reference metrics (#231)\n\n## What changed\n\n- add production readiness, desync response, disconnect/rejoin, and\nmeasured network-tuning guides\n- synchronize every new page into MkDocs and the generated GitHub wiki\n- document pull-based metrics, wire accounting, MTU risk, and\nquality-report cadence\n- update the graphical P2P example to obey WaitRecommendation while\npolling and render live session/peer health\n- fix the example's previously unchecked fallible builder calls\n\n## Why\n\nM6 requires operational guidance and a reference client that\ndemonstrates the behavior applications must ship. The tuning guide\npublishes schema-v2 sweep evidence, including release-mode N=16 4-byte\nand 32-byte rows, instead of relying on qualitative presets alone.\n\n## Impact\n\nNo wire or library API behavior changes. Users gain deployable\noperations guidance; the graphical example now demonstrates time-sync\nbackpressure and metrics monitoring.\n\n## Validation\n\n- cargo clippy --workspace --all-targets --features tokio,json\n- cargo nextest run --no-capture (2,834 passed; 73 skipped)\n- cargo test --doc --features json -- --nocapture (161 passed; 50\nignored)\n- cargo check --example ex_game_p2p --features graphical-examples,json\n- release-mode full_matrix_sweep (1 passed; N=16 rows zero desync)\n- markdownlint, local link check (1,431 links), strict wiki validation,\ndoc-claims, typos, and agent preflight\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Documentation, wiki sync, doc comments, and example-only loop changes;\nno library runtime or protocol logic modified.\n> \n> **Overview**\n> Adds **four new operational guides** (production checklist, desync\nincident playbook, disconnect/rejoin playbook, network tuning with\nchecked-in sweep baselines) and wires them into **MkDocs**, **wiki\nsync**, and **sidebar** navigation.\n> \n> **Telemetry** gains a **Pull-Based Metrics** section documenting\n`metrics()` / `peer_metrics()`, wire-byte accounting, MTU risk counters,\nand quality-report cadence; **See Also** links cross-reference the new\nguides. **`NetworkStats::ping`** doc comments now state updates happen\non the quality-report interval, not per packet.\n> \n> The **`ex_game_p2p`** example propagates fallible `SessionBuilder`\ncalls with `?`, handles **`WaitRecommendation`** by skipping simulation\nticks while still polling, and draws live rollback/lag/stall and\nper-peer ping/kbps/**`sync_health`** overlays.\n> \n> No rollback protocol or wire-format behavior changes beyond\ndocumentation clarity.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n556d6c6381bc9413afcb375a65f2f4a7a2273fad. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\n---------\n\nCo-authored-by: copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>",
          "timestamp": "2026-07-12T16:29:47-07:00",
          "tree_id": "3617d49e6bc2121f1810b50dd9587d8091baa03b",
          "url": "https://github.com/wallstop/fortress-rollback/commit/bb7122b59d9311090065c5bb4ea266bb0bc001fd"
        },
        "date": 1783899288361,
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
            "name": "SyncTestSession/advance_frame_no_rollback/2",
            "value": 122,
            "range": "± 1",
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
            "value": 469,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 747,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1103,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 134342,
            "range": "± 193",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 50853,
            "range": "± 3703",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1405,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1602,
            "range": "± 1",
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
            "name": "sync_layer_noop",
            "value": 0,
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
          "id": "5de83bea40e3a8b32e1d7875d7b3ce997eec6bf7",
          "message": "Harden frame boundaries and benchmark gating (#232)\n\n## What changed\n\n- pin i32 frame saturation and saved-state integrity at the terminal\nframe\n- document checksum-history size-cap retention and cover\nmissing-checksum pruning\n- saturate extreme checksum cadences and retention arithmetic without\nsigned narrowing overflow\n- hard-gate stable microsecond Criterion benchmarks at a 150% threshold\n- keep nanosecond session, input, compression, metrics, and wire-length\ncases informational\n- replace the SyncLayer no-op benchmark with representative save/advance\nwork\n\n## Why\n\nM6 requires deterministic boundary coverage and a performance gate\nstrict enough to catch material regressions without treating\nshared-runner timer noise as a merge blocker.\n\n## Validation\n\n- cargo fmt --all -- --check\n- cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings\n- cargo nextest run --workspace --features tokio,json --no-capture:\n2,866 passed; 73 skipped\n- cargo nextest list --workspace --all-targets --features tokio,json\n- actionlint .github/workflows/ci-benchmarks.yml\n- agent preflight: all checks passed\n- targeted frame, checksum-retention, and extreme-config tests\n- Cursor and Copilot exact-head reviews: zero remaining issues\n\n## Benchmark gate acceptance drill\n\nDraft PR #233 deliberately added 1 ms to Message\nserialization/round_trip_input_msg. Actions run 29214858594 measured\n1,230,741 ns versus the 134,342 ns baseline (9.16x), emitted the\n1.50-threshold performance alert, and failed Run Benchmarks while the\nsmoke job passed. The drill PR was closed without merge and its branch\ndeleted.\n\n---------\n\nCo-authored-by: copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>",
          "timestamp": "2026-07-12T17:27:42-07:00",
          "tree_id": "2109d507ac79456aaaceee75a8390b7e67afa110",
          "url": "https://github.com/wallstop/fortress-rollback/commit/5de83bea40e3a8b32e1d7875d7b3ce997eec6bf7"
        },
        "date": 1783902902030,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 129235,
            "range": "± 5200",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46979,
            "range": "± 247",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1557,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3146,
            "range": "± 214",
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
        "date": 1783914775449,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 132736,
            "range": "± 2921",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46289,
            "range": "± 1694",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 2",
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
            "value": 3010,
            "range": "± 256",
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
        "date": 1783958608182,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 124707,
            "range": "± 735",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46007,
            "range": "± 432",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 91",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3145,
            "range": "± 290",
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
          "id": "f8f0623acbbaab590244f8c913bf75e8665316b8",
          "message": "Confirm H-SKEW phase mechanism (#236)\n\n## Summary\n\n- add an opt-in, bounded phase-resolved sampling mode for skew-gated\nsimulation runs\n- preserve the controller's `frames_ahead` signal in deterministic\nfailure artifacts without changing legacy trace identity\n- convert the one-hour H-SKEW cost inference into a phase-resolved\nverdict\n\n## Why\n\nThe existing one-hour experiment retained only 12 samples. Aggregate\nhistograms suggested that the fast peer paid deeper rollbacks while\ntraversing the 0→3-frame wait-recommendation dead band, but they could\nnot prove the temporal relationship.\n\nThe new 2,457-sample skew trace places all 71 recommendations and 213\nobeyed waits at phase 3. Mean fast-peer rollback depth rises\nmonotonically across phases 0→3 (approximately 1.43, 2.37, 3.33, and\n3.94 frames). This confirms the mechanism for the deliberately\nalways-changing stress input without changing production time-sync\npolicy.\n\n## Validation\n\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- default nextest suite: 2,842 passed, 73 skipped\n- hot-join nextest suite: 3,098 passed, 74 skipped\n- release one-hour H-SKEW probe with exact replay: passed\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes are confined to simulation harness telemetry, artifact\nvalidation, and tests; production rollback/time-sync code is untouched\nand legacy 12-sample traces are unchanged by default.\n> \n> **Overview**\n> Adds an **opt-in** `RunOptions::phase_resolved_control_samples` mode\nfor schema-v16+ `SkewGated60Hz` runs. Default fleet behavior stays on\nthe compact ≤12-sample progress trace; when enabled, the harness records\na **bounded** time series (up to ~4k samples for two players, scaled by\npeer count) that includes **`frames_ahead`** on each sample and extra\nsamples on opportunity-lead / obeyed-wait transitions, without the\nschema-16 endpoint/link gauge payload.\n> \n> **Failure artifacts** validate the larger sample cap, require per-peer\n`frames_ahead` when the flag is set, and skip endpoint/link fields for\nphase-resolved replays. Shrinker remapping preserves the new option.\n> \n> **Tests:** a focused bounded/determinism probe; the hour-equivalent\nH-SKEW experiment now runs with phase sampling and asserts\nphase-bucketed lag stability, that waits/recommendations fire only at\nthe three-frame dead band, and monotonic rollback depth across\ncontroller phases 0→3.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nefd080badcacb616fabed2049c087a32c876667b. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-13T10:26:24-07:00",
          "tree_id": "9febbf4482d925e5feb55202f195fc93c44691f5",
          "url": "https://github.com/wallstop/fortress-rollback/commit/f8f0623acbbaab590244f8c913bf75e8665316b8"
        },
        "date": 1783963997130,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 72561,
            "range": "± 2571",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 31420,
            "range": "± 2610",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 535,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 799,
            "range": "± 57",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 5918,
            "range": "± 244",
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
        "date": 1784048659892,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 127214,
            "range": "± 871",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46561,
            "range": "± 535",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 100",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2807,
            "range": "± 228",
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
        "date": 1784054721120,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 106583,
            "range": "± 600",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 39284,
            "range": "± 302",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 743,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 991,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 7553,
            "range": "± 115",
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
        "date": 1784061854453,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 123483,
            "range": "± 1282",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46605,
            "range": "± 271",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 88",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3145,
            "range": "± 212",
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
        "date": 1784175805172,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 138537,
            "range": "± 919",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46587,
            "range": "± 367",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 93",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3133,
            "range": "± 202",
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
        "date": 1784178493304,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 127195,
            "range": "± 2461",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 47136,
            "range": "± 908",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1245,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1558,
            "range": "± 109",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3136,
            "range": "± 212",
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
        "date": 1784223603253,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 143845,
            "range": "± 4218",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 48211,
            "range": "± 1734",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 105",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3148,
            "range": "± 224",
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
        "date": 1784230911129,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 139595,
            "range": "± 8053",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 50218,
            "range": "± 255",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1406,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1604,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3127,
            "range": "± 239",
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
        "date": 1784339744171,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 131543,
            "range": "± 480",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 50872,
            "range": "± 357",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1405,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1603,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3108,
            "range": "± 239",
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
        "date": 1784392097828,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 131494,
            "range": "± 2230",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 50908,
            "range": "± 387",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1405,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1603,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3125,
            "range": "± 356",
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
        "date": 1784398803841,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 133248,
            "range": "± 361",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 51472,
            "range": "± 568",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1405,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1601,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3109,
            "range": "± 238",
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
        "date": 1785082679926,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 117558,
            "range": "± 3308",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 39923,
            "range": "± 450",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 761,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 991,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 7621,
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
          "id": "88b6c3040482e05118caf56279b638dbd8ad3982",
          "message": "Promote N=16 controls and refresh dependency CI (#267)\n\n## What\n\nPromotes the four coordinated A8 N=16 simulation controls into the\nnormal PR test suite:\n\n- smoke-fleet invariants\n- reliable-FIFO/TCP invariants\n- seeded-divergence oracle coverage\n- bit-identical trace determinism\n\nThe N=12 smoke probe and 5,000-step N=16 budget diagnostic remain manual\nand ignored.\n\nThis PR also closes the dependency-maintenance work exposed by the\npromotion:\n\n- aligns `wasm-bindgen-cli` with locked `wasm-bindgen 0.2.126`\n- preserves Rust 1.86 compatibility with `serial_test 3.5.0` and a\nDependabot semver-major guard\n- refreshes all 65 compatible Cargo packages (`cargo update --dry-run`\nnow locks 0 packages)\n- pins `taiki-e/install-action@v2.85.6` and `docker/login-action@v4.6.0`\n- gives the promoted smoke control an evidence-backed 180-second local\nNextest ceiling, with retries still disabled\n- gives tarpaulin a 300-second response timeout inside the existing\n30-minute job cap\n\n## Why\n\nA8 required both an approximately 10-second PR budget and seven\nconsecutive scheduled `main` successes. The release-mode promotion set\nruns in 4.250 seconds locally, and the scheduled simulation fleet\ncompleted eight uninterrupted successful runs from 2026-07-25 through\n2026-08-01.\n\nThe initial PR runs then exposed three pre-existing CI incompatibilities\nfrom the preceding dependency update: a wasm-bindgen schema mismatch, an\nMSRV-incompatible serial_test major, and tarpaulin's 60-second\ninstrumented-response default. Each was reproduced before repair.\n\n## Impact\n\nNo production, public API, wire-format, allocation, or session-runtime\nbehavior changes. Changes are limited to test promotion, CI/tool\nconfiguration, dependency resolution, and regression contracts. No\nchangelog entry is required.\n\n## Review readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS (no production diff)\n- Determinism: PASS\n- Agent preflight: PASS\n- MSRV and supply-chain checks: PASS\n- Error handling: N/A\n- Changelog: N/A\n\n## Validation\n\n- release-mode promotion set: 4/4 passed in 4.250 seconds\n- default Nextest: 2,882/2,882 passed\n- hot-join Nextest: 3,138/3,138 passed\n- focused loaded N=16 smoke: passed in 60.624 seconds on its first\nattempt under the new 180-second ceiling\n- `cargo +1.86 check --all-targets`\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`\n- `cargo deny check`\n- `cargo fmt --check`\n- warning-denied docs with `hot-join,tokio,json`\n- 1,969/1,969 script tests\n- actionlint and agent preflight, including 66 CI toolchain contract\ntests\n- real wasm browser smoke with CLI 0.2.126: 1/1 passed\n- exact tarpaulin command: 53.54% line coverage (7,034/13,139)\n- two-round main-thread adversarial review; repeat sweep clean\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> No production code changes, but PR CI now depends on longer simulation\nruns and a large lockfile refresh; serial_test 3.x and refreshed\ntransitive deps could affect test ordering/isolation if misused.\n> \n> **Overview**\n> Promotes **four A8 N=16 simulation controls** from ignored/manual runs\ninto the normal PR suite: smoke-fleet invariants, TCP reliable-FIFO\ninvariants, seeded oracle divergence, and bit-identical trace\ndeterminism. The N=12 smoke probe and other large-mesh diagnostics stay\n**`#[ignore]`**.\n> \n> **CI and tooling** are adjusted so those tests and the dependency\nrefresh stay green: a **180s** Nextest slow-timeout override for the\npromoted smoke probe, **300s** tarpaulin `--timeout` for instrumented\nN=16 work, **`wasm-bindgen-cli` pinned to locked 0.2.126** (with\n**`taiki-e/install-action@v2.85.6`**), **`serial_test` held at 3.5** for\nRust **1.86** MSRV plus a Dependabot ignore for `serial_test`\nsemver-major bumps, and a broad **`Cargo.lock`** refresh with\n**`docker/login-action@v4.6.0`**. Workflow regression tests assert\ntarpaulin timeout and wasm-bindgen lock alignment.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n2855d350eeaa818d518e3ea85dd4a62d339d9807. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-01T15:56:54-07:00",
          "tree_id": "7ff864f4c78d16393dabc944a3b64b1651e14682",
          "url": "https://github.com/wallstop/fortress-rollback/commit/88b6c3040482e05118caf56279b638dbd8ad3982"
        },
        "date": 1785625533747,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 131421,
            "range": "± 494",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 47798,
            "range": "± 130",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3147,
            "range": "± 250",
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
        "date": 1785631647855,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 134954,
            "range": "± 315",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 50499,
            "range": "± 355",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1405,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1601,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3111,
            "range": "± 254",
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
        "date": 1785634362787,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 121767,
            "range": "± 3433",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 41107,
            "range": "± 1078",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 799,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1020,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 7792,
            "range": "± 142",
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
        "date": 1785637791937,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 131341,
            "range": "± 2409",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 47751,
            "range": "± 425",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 92",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2787,
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
          "distinct": true,
          "id": "df198e728fb3b87e0ba5cd3914bb65f0d07479d0",
          "message": "chore(ci): refresh current action pins (#271)\n\nUpdate all sccache-action call sites to v0.0.11 and pin setup-java to v5.7.0 while preserving the publish workflow's immutable SHA policy.\n\nSupersedes #269.",
          "timestamp": "2026-08-01T19:48:51-07:00",
          "tree_id": "6711ce26741e8a72cf3e891561326fe757905de0",
          "url": "https://github.com/wallstop/fortress-rollback/commit/df198e728fb3b87e0ba5cd3914bb65f0d07479d0"
        },
        "date": 1785639443882,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 143131,
            "range": "± 6138",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 47184,
            "range": "± 1599",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 108",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3146,
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
          "id": "48532d1f5613cc3a4528cffc0ba76199f16dd8bc",
          "message": "Eliminate warmed sync-test input staging allocations (#272)\n\n## What\n\n- replace per-frame `BTreeMap` input staging in `SyncTestSession` with\nfallibly preallocated player-indexed slots\n- retain slot storage across frames while preserving duplicate\noverwrite, missing-input retry, and deterministic player ordering\n- tighten the deterministic allocation ledger to zero operations/bytes\nfor warmed N=2 and N=4 frames, and one exact 128-byte\nreturned-`InputVec` spill at N=16\n- document the observable performance improvement in the changelog\n\n## Why\n\nPR #270 established a deterministic allocation ledger and measured one\nallocation / 192 bytes per warmed frame at N=2 and N=4, plus four\noperations / 800 bytes at N=16. Those allocations came from rebuilding\nthe sync-test staging map every frame. Constructor-owned slots remove\nthat repeated work without changing the public API or wire behavior.\n\nProgresses #264.\n\n## Impact\n\nWarmed `SyncTestSession::advance_frame()` calls for one-to-four-player\nsessions are heap-allocation-free. Sixteen-player frames now allocate\nonly for the returned `InputVec` above its inline capacity. Construction\nremains fallible for arbitrary configured player counts, and an\nimpossible slot-length mismatch returns a structured internal error.\n\n## Validation\n\n- allocation contract: 10 consecutive debug runs and one release run\n- focused semantic/allocation tests: 3/3\n- sync-test unit suite: 49/49\n- sync-test determinism regression: 1/1\n- default Nextest matrix: 2,883/2,883\n- `hot-join` Nextest matrix: 3,139/3,139\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- warning-denied workspace documentation\n- changelog formatting, Unreleased policy, and version synchronization\n- agent preflight, including 286 release-automation tests and 1,393\nlinks\n- independent adversarial review: zero findings\n\n## Review Readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS\n- Determinism: PASS\n- Agent preflight: PASS\n- Error handling: PASS\n- Tests breadth: PASS\n- Design log reviewed: N/A (private staging refactor)\n- CHANGELOG reviewed: YES\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Private sync-test staging only; semantics are covered by unit and\nallocation-contract tests with no public API or network wire changes.\n> \n> **Overview**\n> **`SyncTestSession`** no longer rebuilds per-frame input staging with\na **`BTreeMap`**. Construction fallibly reserves\n**`Vec<Option<PlayerInput>>`** indexed by player handle;\n**`add_local_input`** overwrites slots, and **`advance_frame`** clears\nslots to **`None`** while keeping that storage for the next frame.\n> \n> **Behavior preserved or tightened:** missing-input checks cover empty\nslots; player-index ordering in **`AdvanceFrame`** inputs is unchanged;\na failed **`advance_frame`** keeps already-staged inputs so callers can\nadd only missing players and retry.\n> \n> **Allocation impact:** warmed **`advance_frame()`** for 1–4 players is\nheap-free; 16-player frames are limited to the returned **`InputVec`**\nspill above inline capacity (allocation contract and changelog updated).\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\ne75047918c901b6ab1d3efd941014a20544284f7. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-01T21:10:58-07:00",
          "tree_id": "9ec5f320b3389a097eb8fdaa21a7fc088ad6c7a0",
          "url": "https://github.com/wallstop/fortress-rollback/commit/48532d1f5613cc3a4528cffc0ba76199f16dd8bc"
        },
        "date": 1785644432357,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 146309,
            "range": "± 4246",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 54020,
            "range": "± 367",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 105",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2775,
            "range": "± 342",
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
        "date": 1785687119048,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 135364,
            "range": "± 751",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 51920,
            "range": "± 272",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1405,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1601,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3139,
            "range": "± 241",
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
        "date": 1785687492328,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 132653,
            "range": "± 795",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 48373,
            "range": "± 290",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1243,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 109",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3140,
            "range": "± 204",
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
        "date": 1785691374075,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 129935,
            "range": "± 4382",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 51528,
            "range": "± 329",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1406,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1602,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3122,
            "range": "± 256",
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
        "date": 1785694668732,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 144885,
            "range": "± 1766",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 48449,
            "range": "± 1122",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 89",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3140,
            "range": "± 204",
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
        "date": 1785712459196,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 135324,
            "range": "± 1737",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46345,
            "range": "± 258",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1557,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3152,
            "range": "± 243",
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
        "date": 1785714305845,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 146873,
            "range": "± 2685",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46283,
            "range": "± 324",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 86",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3140,
            "range": "± 201",
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
        "date": 1785716600196,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 147837,
            "range": "± 1644",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 46749,
            "range": "± 213",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 107",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3146,
            "range": "± 207",
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
        "date": 1785740381307,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 121847,
            "range": "± 523",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 36493,
            "range": "± 131",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1245,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 81",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3158,
            "range": "± 215",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}