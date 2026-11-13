window.BENCHMARK_DATA = {
  "lastUpdate": 1794552780090,
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
        "date": 1785773282623,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 124150,
            "range": "± 6313",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 36289,
            "range": "± 197",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1405,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1602,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3115,
            "range": "± 248",
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
        "date": 1786222918693,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 92683,
            "range": "± 2737",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 28521,
            "range": "± 1033",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 643,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 853,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 6509,
            "range": "± 364",
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
        "date": 1787511297101,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 51322,
            "range": "± 1230",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 42605,
            "range": "± 565",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1406,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1603,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3132,
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
          "id": "2363992a728cdfcb14fb67f2ae3ef9a7f75be1e4",
          "message": "perf: streamline SyncTest and harden mutation CI (#301)\n\n## Summary\n\n- refresh all four Cargo workspace lockfiles through compatible\nminor/patch releases, with `syn` advanced to 3.0.4 and MSRV-blocked\nmajors explicitly left in place\n- add a hermetic pip-vs-uv benchmark, informational quality job,\nartifact, and manual pre-commit hook for #281\n- profile and streamline `SyncTestSession::advance_frame`: prune\nchecksum history once and avoid allocating a mismatch vector on the\nconsistent hot path\n- repair mutation CI after the latest-main timeout: pin cargo-mutants\n27.1.0, use exact-head diff scope for PR/push, dynamically shard the\n4,696-mutant full corpus on Linux, exclude formal-proof-only false\nmutants, and aggregate every shard fail-closed\n- complete the #287 tool-gap audit; track the two real follow-ups\nseparately in #299 (incremental Python typing) and #300\n(standalone-workspace supply-chain coverage)\n\n## Latest-main RCA\n\nThe post-merge main audit found 18 successful workflow runs plus one\ncancelled mutation run. The old workflow serially repeated 5,668 mutants\non Linux, macOS, and Windows; included 972 Kani-only mutants invisible\nto ordinary tests; used derived per-mutant timeouts up to 1,055 seconds;\nfloated cargo-mutants; passed a list-only runtime flag; parsed the wrong\nreport path; and swallowed failures.\n\nThis PR replaces that topology with one authoritative baseline and\nbounded exact-head shards. Diff/filtered runs reject every missed or\ntimed-out mutant. Full sweeps reject every timeout and require at least\nan 80% global mutation score. Missing artifacts, incomplete shard\nindexes/counts, contradictory exit statuses, infrastructure failures,\nand cancelled dependencies all fail the summary.\n\n## Validation\n\n- `cargo fmt --all -- --check`\n- strict workspace Clippy with `tokio,json`\n- default Nextest: 2,999 passed, 71 skipped\n- hot-join Nextest: 3,257 passed, 72 skipped\n- complete Python suite: 2,083 passed\n- mutation workflow/aggregator contracts: 14 passed\n- exact local mutation diff: 25 mutants; 10 caught, 15 unviable, 0\nmissed, 0 timed out\n- full production mutation listing: 4,696; 0 Kani/proof and 0\n`proof_vec` leaks\n- `cargo deny check` and `cargo audit --deny warnings`\n- `python3 scripts/ci/agent-preflight.py --all`\n- actionlint, workspace lock freshness, rustdoc, and doctests\n- final adversarial re-review: zero remaining findings\n\nCloses #281.\nCloses #287.\nAdvances #297; the remaining P2P/spectator sweep stays open.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Changes rollback checksum verification in\n`SyncTestSession::advance_frame` (two-pass logic with documented\nside-effect idempotency) plus CI now gates PRs on mutation results—both\naffect determinism testing and merge policy.\n> \n> **Overview**\n> **SyncTest hot path:** `SyncTestSession::advance_frame` now prunes\nchecksum history **once** per advance (via `prune_checksum_history`)\ninstead of repeating an O(history) `BTreeMap` retain on every frame in\nthe window, and uses an **`any()` probe** so the common all-consistent\ncase avoids building a mismatch `Vec`. Mismatch reporting and rollback\nbehavior are unchanged; new regression tests lock down multi-frame\n`MismatchedChecksum` payloads and pruning bounds.\n> \n> **Mutation CI:** Replaces the cross-platform serial workflow with a\n**baseline + dynamically sharded Linux** pipeline: PR/push runs\n**`--in-diff`** on changed `*.rs`, pins **cargo-mutants 27.1.0**, uses\nper-mutant timeouts and fail-closed aggregation via\n**`aggregate_mutation_results.py`** (zero missed/timeouts on\ndiff/filtered; ≥80% score on full sweeps). **Kani-only** code is\nexcluded from mutation (`proof_vec.rs`, `exclude_re`, `mutants::skip` on\nproof modules).\n> \n> **Tooling / evidence:** Adds a hermetic **pip vs uv** benchmark\nscript, informational quality CI job, manual pre-commit hook, and\ncontract tests; updates code-review guidance on iterator `clone()` vs\ncollection clone; folds min/max lag in simulation fleet tests to avoid\nredundant iterator clones. Lockfiles refresh minor/patch deps.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nce523fc26d521a5a5db61d036516ca654f03059f. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-24T13:35:07-07:00",
          "tree_id": "206ec0183013bf3431ff81d54e8b1fdc1a4a8c7a",
          "url": "https://github.com/wallstop/fortress-rollback/commit/2363992a728cdfcb14fb67f2ae3ef9a7f75be1e4"
        },
        "date": 1787604187841,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 38902,
            "range": "± 122",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 31189,
            "range": "± 120",
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
            "value": 2764,
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
          "distinct": true,
          "id": "0370e8a7dd71e59d6f72d89d14feeebec6f7a382",
          "message": "perf: close the open hardening and usability backlog (#304)\n\n## Summary\n\nThis branch closes the complete paginated open-issue set in\ngameplay-impact order:\n\n- removes measured P2P and spectator hot-path overhead while preserving\nunknown-source diagnostics;\n- replaces fragmented onboarding with one runnable first-session path\nand short route maps;\n- reduces the crates.io archive from 91 to 58 files and from 1,091,787\nto 914,854 compressed bytes;\n- audits all four authoritative Cargo workspaces in hosted\nvulnerability, deny-policy, license, and freshness jobs;\n- type-checks all 58 production Python modules with strict, pinned mypy\nat the Python 3.10 floor.\n\nIt also raises the docs tooling floor to the latest compatible\npymdown-extensions 11.0.2.\n\n## Measured results\n\n- All-local N=2 P2P callgrind: 743,107,916 → 627,307,916 instructions\n(-15.58%).\n- Warm spectator polling: zero allocations for one- and four-host rows.\n- Per-packet connection-status fan-out: zero clone allocations at\nN=2/4/16.\n- Published archive: 36.26% fewer files, 11.57% fewer unpacked bytes,\n16.21% fewer compressed bytes.\n- Strict mypy: 34 initial diagnostics across 15 files → zero issues in\n58 files.\n- Strict pyright comparison: 495 errors on the same scope, so one\nchecker is maintained.\n\n## Verification\n\n- Latest `main` head `2363992`: all 16 workflow groups successful.\n- Strict Clippy: `tokio,json` and `hot-join,tokio,json` matrices pass.\n- Nextest: 2,903 default and 3,161 hot-join tests pass.\n- Rustdoc: 169 pass, 54 configured ignores.\n- Python: 2,098 tests pass; Ruff and strict mypy pass.\n- Package clean-room verification and all 1,953 packaged library tests\npass.\n- All four Cargo locks pass RustSec and cargo-deny policy; all\ncompatible lock resolutions are current.\n- MkDocs strict build, Markdownlint, wiki validation, actionlint, and\nagent preflight pass.\n- Main-thread adversarial review found and fixed the package fixture\nomission, global license-exception noise, cargo-deny config resolution,\npassive contributor prose, and one stale release packaging assertion; no\nfindings remain.\n\n## Dependency disposition\n\nAll compatible Cargo, npm, docs, and release-tooling dependencies are\ncurrent. The only newer Cargo packages exceed the Rust 1.86 MSRV:\nbincode-next 3.1.1 requires Rust 1.90, and serial_test 4.0.1 requires\nRust 1.93.1.\n\nCloses #297\nCloses #299\nCloses #300\nCloses #302\nCloses #303\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Changes core P2P/protocol event handling and crates.io publish\ncontents, though allocation contracts and expanded supply-chain CI\nreduce regression risk.\n> \n> **Overview**\n> This PR tightens **runtime networking**, **consumer packaging**,\n**supply-chain CI**, **Python tooling**, and **onboarding docs** in one\npass.\n> \n> **Performance:** Input events now carry `peer_connect_status` as a\nshared `Arc<[ConnectionStatus]>` so one packet does not clone an\nN-player status vector per player. All-local `P2PSession` polls skip\nendpoint work after socket accounting (unknown-source diagnostics stay).\nSpectator polls reuse constructor-reserved scratch for disconnect\nbatches instead of allocating each idle tick. Allocation contracts and a\nnew `p2p_all_local` benchmark lock in zero- or bounded-allocation\nbehavior on warmed paths.\n> \n> **crates.io:** `Cargo.toml` switches from a long `exclude` list to an\nanchored **`include`** allowlist (sources, licenses, README, one wire\ngolden test) so published tarballs stay small and self-testable; README\nlinks are adjusted for crates.io.\n> \n> **CI / supply chain:** Security and scheduled freshness jobs run in a\n**four-workspace matrix** (root, fuzz, loom, godot-emscripten) with\n`cargo audit`, workspace-scoped `cargo deny`, and **lockfile-scoped\nlicense exceptions** (NCSA/MPL test-only deps). Freshness reports are\nrequired; version lag stays advisory. Quality CI adds **blocking strict\nmypy** (Python 3.10 floor, pinned 2.3.1) plus a matching pre-commit hook\nfor production `scripts/`.\n> \n> **Docs:** README and docs home become short route maps; new **Getting\nStarted** is the single runnable `SyncTestSession` path (MkDocs/wiki nav\nupdated). User guide quick-start defers to that guide.\n> \n> **Tests:** New contracts cover CI security matrices, package payload,\nPython typing gate, and docs onboarding shape.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n579a4098729acf5121631f667076b3d623ca971d. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-24T18:08:57-07:00",
          "tree_id": "c6565a10cf05f80062f2ac98d0fd475db5fa96c8",
          "url": "https://github.com/wallstop/fortress-rollback/commit/0370e8a7dd71e59d6f72d89d14feeebec6f7a382"
        },
        "date": 1787620651392,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 51402,
            "range": "± 231",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 41034,
            "range": "± 263",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1406,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1601,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3138,
            "range": "± 272",
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
        "date": 1787673895662,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 52050,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 42690,
            "range": "± 219",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1406,
            "range": "± 10",
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
            "value": 3137,
            "range": "± 237",
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
        "date": 1787846739267,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 35985,
            "range": "± 1523",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 32288,
            "range": "± 306",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 655,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 842,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 6580,
            "range": "± 126",
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
        "date": 1787868801470,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 59231,
            "range": "± 705",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 42558,
            "range": "± 386",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1863,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 99",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2772,
            "range": "± 223",
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
        "date": 1787899204992,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 51391,
            "range": "± 202",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 41613,
            "range": "± 312",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1406,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1602,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3220,
            "range": "± 232",
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
          "id": "e894056c153ba1b78bdb2e1ae3cd1b4db40f1bbb",
          "message": "chore(deps): update Z3 verification stack (#319)\n\n## Summary\n\n- update the optional `z3` dependency to 0.21.0 and migrate the bundled\ncompatibility feature to upstream's `vendored` path\n- pin Z3 5.1.0 for hosted verification and the devcontainer, including\nexplicit build/runtime library selection\n- add static regression coverage and document the solver compatibility\nfloor, recommended version, and migration path\n\n## Dependency audit\n\n- all four Cargo workspaces select zero further compatible updates\n- npm reports no outdated packages\n- `bincode-next` 3.1.1 remains blocked by its Rust 1.90 requirement\n- `serial_test` 4.0.1 remains blocked by its Rust 1.93.1 requirement\n\n## Validation\n\n- `cargo fmt --check`\n- strict workspace/all-target Clippy with `tokio,json`\n- feature-specific Nextest: 3,008 passed, 72 skipped\n- `cargo c`\n- `cargo t`: 2,975 passed, 72 skipped\n- native Z3 5.1.0 verification: 65 passed, including 54 Z3 proofs\n- vendored Z3 verification target compiles with `--locked`\n- doc tests: 160 passed, 50 ignored\n- rustdoc with warnings denied\n- focused CI/devcontainer tests: 101 passed\n- agent preflight: 286 + 66 checks passed\n- Actionlint, wiki consistency, Markdown lint, link check, spell check,\nCargo audit, and Cargo deny\n\n## Risk\n\nThis only changes development-time formal verification and tooling.\nProduction Rust, runtime public APIs, network/replay formats,\ndeterministic state, and allocation-sensitive paths are unchanged.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes affect optional formal-verification features, CI, and\ndevcontainer tooling only—not production runtime APIs, networking, or\nreplay formats.\n> \n> **Overview**\n> Upgrades the optional **`z3`** crate to **0.21** and documents a\n**breaking** change for the dev-only **`z3-verification`** feature:\nnative Z3 must be **≥4.13.3**, with **5.1.0** recommended so Rust\nbindings match Z3 5.x enums. **`z3-verification-bundled`** now enables\nupstream’s **`vendored`** build instead of **`bundled`**.\n> \n> **CI and devcontainer** stop using distro **`libz3-dev`** and instead\ninstall pinned **`z3-solver==5.1.0.0`**, assert the runtime version,\nexport **`Z3_LIBRARY_PATH_OVERRIDE`**, **`LD_LIBRARY_PATH`**, and\n**`Z3_SYS_Z3_VERSION`**, and run Z3 Nextest with **`--locked`**. The\ndevcontainer registers the wheel’s **`lib`** via **`ld.so.conf.d`** and\n**`ldconfig`**.\n> \n> **Semver** job timeout increases from **20** to **40** minutes so a\ncold lockfile rebuild after the dependency bump can finish and save\ncache instead of cancelling. Static workflow tests lock in the Z3 pin\nand semver timeout.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\na3c86fc5afbae144d8542251cea2524472d02355. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-08-28T13:38:49-07:00",
          "tree_id": "86f948e4d1dabc16d316266bfce304e0dab27112",
          "url": "https://github.com/wallstop/fortress-rollback/commit/e894056c153ba1b78bdb2e1ae3cd1b4db40f1bbb"
        },
        "date": 1787950040765,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 59162,
            "range": "± 351",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 41558,
            "range": "± 436",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1860,
            "range": "± 111",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1557,
            "range": "± 103",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2987,
            "range": "± 224",
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
          "id": "d5ac4e053f1d457e8f74b5363e08b368f17500ea",
          "message": "chore(deps): bump the cargo-workspace group with 2 updates (#334)\n\nBumps the cargo-workspace group with 2 updates:\n[smallvec](https://github.com/servo/rust-smallvec) and\n[z3](https://github.com/prove-rs/z3.rs).\n\nUpdates `smallvec` from 1.16.1 to 1.16.2\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/servo/rust-smallvec/releases\">smallvec's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v1.16.2</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Backport v2’s retain implementation to v1 by <a\nhref=\"https://github.com/charliermarsh\"><code>@​charliermarsh</code></a>\nin <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/pull/596\">servo/rust-smallvec#596</a></li>\n<li>Fix element ownership tracking with may_dangle by <a\nhref=\"https://github.com/charliermarsh\"><code>@​charliermarsh</code></a>\nin <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/pull/594\">servo/rust-smallvec#594</a></li>\n<li>Fix Cargo manual_readme warning by <a\nhref=\"https://github.com/Rayan-and-beyond\"><code>@​Rayan-and-beyond</code></a>\nin <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/pull/608\">servo/rust-smallvec#608</a></li>\n<li>chore: bump version by <a\nhref=\"https://github.com/alejandro-vaz\"><code>@​alejandro-vaz</code></a>\nin <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/pull/617\">servo/rust-smallvec#617</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a\nhref=\"https://github.com/Rayan-and-beyond\"><code>@​Rayan-and-beyond</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/pull/608\">servo/rust-smallvec#608</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/servo/rust-smallvec/compare/v1.16.1...v1.16.2\">https://github.com/servo/rust-smallvec/compare/v1.16.1...v1.16.2</a></p>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/ccf5fc71044d491c46a3d79e7ed53948e6da1590\"><code>ccf5fc7</code></a>\nchore: bump version (<a\nhref=\"https://redirect.github.com/servo/rust-smallvec/issues/617\">#617</a>)</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/af207ccb68496e112f3d7366a66c07c6107ef04a\"><code>af207cc</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/issues/608\">#608</a>\nfrom Rayan-and-beyond/fix/manual-readme-warning-606</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/cda4b73506acb35823a8147ef4397375f7063b82\"><code>cda4b73</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/issues/594\">#594</a>\nfrom astral-sh/charlie/codex-fix-may-dangle</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/d0556cb8e33b02b434074896f753f5f03b20c278\"><code>d0556cb</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/servo/rust-smallvec/issues/596\">#596</a>\nfrom astral-sh/charlie/codex-v1-compact</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/f73914cf4b523741b9af5d13a98e77d59c54bc13\"><code>f73914c</code></a>\nFlatten retain tests into the unit test module</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/954d599e41b074282857e97c44211f0f797e6597\"><code>954d599</code></a>\nMove retain tests into the unit test module</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/8d936338dbcefa380b4dd596379b57582ed5d616\"><code>8d93633</code></a>\nRemove added retain benchmark harness</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/88c6bfabd610f34f4f1f9d530f58fb8a74b9a807\"><code>88c6bfa</code></a>\nLimit compaction optimization to retain</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/b9ef17da33c19cb1d11190e0c7c29e459297cd6a\"><code>b9ef17d</code></a>\nFix element ownership tracking with may_dangle</li>\n<li><a\nhref=\"https://github.com/servo/rust-smallvec/commit/42029c27c9243d8a178a0d64464185547b3a7d6f\"><code>42029c2</code></a>\nCompact retained elements directly in retain and dedup_by</li>\n<li>See full diff in <a\nhref=\"https://github.com/servo/rust-smallvec/compare/v1.16.1...v1.16.2\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\nUpdates `z3` from 0.21.0 to 0.21.1\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/ffb47c0c2d30259836b93b7dfb01679c4eabbff5\"><code>ffb47c0</code></a>\nchore: release (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/588\">#588</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/cd52e5c1e4178193272742a20cc9ca6f2fa4eb92\"><code>cd52e5c</code></a>\nchore: turn off semver-check for z3 crate</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/fdf3e46f85cff7381af3cbf43816305365da7d36\"><code>fdf3e46</code></a>\nchore: disable semver-checks for z3-sys</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/30578dbab75aacb762b7b3761a72b170f992b909\"><code>30578db</code></a>\nfix: Fall back to pkg-config on build feature conflict (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/595\">#595</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/85d2a48bce50478dae15841d8db7d4ffce337b89\"><code>85d2a48</code></a>\nfeat: Improve Windows cross-compilation support (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/592\">#592</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/f5e0ba7b69582630dd72ffe6057401420f23a4c7\"><code>f5e0ba7</code></a>\nfix: Use wrap for Goal's clone impl (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/594\">#594</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/3d9064797f210d5f59c1587bf29b3b4c783edfd1\"><code>3d90647</code></a>\nfeat: FuncDecl::as_array() (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/591\">#591</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/5d4d57ad3a16aa74303b4a23f02918cb9fd20461\"><code>5d4d57a</code></a>\nrefactor: Replace internal usages of z3_ctx.0 with z3_ctx.as_ptr() (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/587\">#587</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/prove-rs/z3.rs/compare/z3-v0.21.0...z3-v0.21.1\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore <dependency name> major version` will close this\ngroup update PR and stop Dependabot creating any more for the specific\ndependency's major version (unless you unignore this specific\ndependency's major version or upgrade to it yourself)\n- `@dependabot ignore <dependency name> minor version` will close this\ngroup update PR and stop Dependabot creating any more for the specific\ndependency's minor version (unless you unignore this specific\ndependency's minor version or upgrade to it yourself)\n- `@dependabot ignore <dependency name>` will close this group update PR\nand stop Dependabot creating any more for the specific dependency\n(unless you unignore this specific dependency or upgrade to it yourself)\n- `@dependabot unignore <dependency name>` will remove all of the ignore\nconditions of the specified dependency\n- `@dependabot unignore <dependency name> <ignore condition>` will\nremove the ignore condition of the specified dependency and ignore\nconditions\n\n\n</details>\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Patch-only lockfile updates with no direct code changes; z3 affects\noptional verification builds only.\n> \n> **Overview**\n> Bumps workspace dependencies in **`Cargo.lock`** only: **`smallvec`**\n1.16.1 → 1.16.2 and **`z3`** 0.21.0 → 0.21.1 (with **`z3-sys`** 0.13.0 →\n0.13.1). No application or manifest changes in this diff.\n> \n> **smallvec** picks up v1 bugfixes (e.g. `retain`/`dedup_by` compaction\nand `may_dangle` ownership tracking). **z3** is a patch release with\nbuild/cross-compile fixes and minor API tweaks upstream; behavior for\noptional Z3 verification tests should stay the same aside from those\ndependency fixes.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nb8922e0add3a38378c7d03b8f5b3cb083cafd628. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-10-03T03:02:26Z",
          "tree_id": "4febf099b20b9b62ce19c164a532f6b17258ac6b",
          "url": "https://github.com/wallstop/fortress-rollback/commit/d5ac4e053f1d457e8f74b5363e08b368f17500ea"
        },
        "date": 1790997421665,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 69231,
            "range": "± 972",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 34960,
            "range": "± 910",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1679,
            "range": "± 110",
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
            "value": 2786,
            "range": "± 414",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "wallstop@wallstopstudios.com",
            "name": "wallstop",
            "username": "wallstop"
          },
          "committer": {
            "email": "wallstop@wallstopstudios.com",
            "name": "wallstop",
            "username": "wallstop"
          },
          "distinct": false,
          "id": "0720b06e73b64d79419389de594e4fdc6d6c7e98",
          "message": "ci(codeql): raise the analyze time budget above the 30-minute boundary\n\nRust CodeQL analysis compiles the whole crate; the last two runs took\n28m37s and then exceeded the 30-minute job cap. Raise the budget to 60\nminutes and assert a margin in the workflow contract test so the gate\nstops flaking on slower runners.",
          "timestamp": "2026-11-01T22:19:05-08:00",
          "tree_id": "80ab7833d2f253cb1d866ff93335b7a68d5d096c",
          "url": "https://github.com/wallstop/fortress-rollback/commit/0720b06e73b64d79419389de594e4fdc6d6c7e98"
        },
        "date": 1793603178166,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 50494,
            "range": "± 236",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 33049,
            "range": "± 687",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1407,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1603,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3250,
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
          "distinct": true,
          "id": "966a95b83ef1c38ee175dd9edebcf597cf78644f",
          "message": "fix(devcontainer): harden MCP tooling and refresh consistency evidence (#358)\n\n## Summary\n\n- Make project MCP credentials optional and independently configurable,\nadd a secret-safe health report, and diagnose stale file/process\nenvironment drift.\n- Migrate devcontainer OpenCode installation and the\nbounded-network-retry plugin to OpenCode 2, including legacy V1 cleanup\nand interruption recovery.\n- Refresh all MSRV-compatible Cargo lockfile dependencies and prevent\nknown-incompatible Dependabot release lines from recurring.\n- Define the confirmed-prefix consistency/availability contract and\nrevalidate the schema-v3 simulation baseline at current HEAD.\n\n## Consistency evidence\n\n- `sweep_pr_gate`: 10/10 cells matched every checked-in baseline field\nexactly; both replay checks passed; zero desyncs.\n- Release `full_matrix_sweep`: 136/136 cells passed at 5,000 steps and\nseed 1 across N=2/3/4/8/12/16; zero desyncs.\n- The tuning guide now distinguishes confirmed consistency, speculative\nresponsiveness, and confirmed availability, and states\ndeterministic-game, polling, delivery, and CAP assumptions.\n- Reorder, duplication, burst, asymmetry, partition/flap, and poll-stall\nevidence remain explicitly queued in `PLAN.md`; this PR does not claim\nthose axes are covered.\n\n## Dependency disposition\n\nAll four Cargo workspaces now select zero compatible updates. Remaining\nnewer releases exceed the Rust 1.86 MSRV: bincode-next 3.1.1 (1.90),\nparking_lot 0.13 (1.95), proptest 1.12 and z3 0.22 (1.88), and\nserial_test 4.0.1 (1.93.1). Dependabot holds those release lines while\nallowing compatible patches.\n\n## Validation\n\n- `bash scripts/tests/test-ai-backends.sh` — pass\n- `.venv/bin/pytest scripts/tests/test_codex_devcontainer_setup.py -q` —\n48 passed\n- `.venv/bin/pytest scripts/tests -q` — 2,186 passed\n- `cargo clippy --workspace --all-targets --features tokio,json` — pass\n- `cargo nextest run --no-capture` — 2,975 passed, 72 skipped\n- `cargo +1.86.0 check --locked --all-targets` — pass\n- `RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps` — pass\n- `cargo test --doc -- --nocapture` — 160 passed, 50 ignored\n- `python3 scripts/release/workspace_locks.py check` — all four locks\npass\n- `actionlint`, shell portability, Markdownlint, wiki sync/consistency,\nlink checks, doc claims, typos, plan hygiene, and changed-file-aware\nagent preflight — pass\n\nNo changelog entry: tooling, documentation, and compatible lockfile-only\nchanges do not alter the public crate API, wire format, or library\nbehavior.\n\n## Operational note\n\nReal credentialed MCP health verification still requires the owner's\n`.env.local` and a devcontainer rebuild. The health report intentionally\nexits `2` with setup guidance when optional credentials are absent.",
          "timestamp": "2026-11-04T13:04:50-08:00",
          "tree_id": "f4685543acd26b1bf457227dd4a8e0f1ef910ea1",
          "url": "https://github.com/wallstop/fortress-rollback/commit/966a95b83ef1c38ee175dd9edebcf597cf78644f"
        },
        "date": 1793826826749,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 50043,
            "range": "± 526",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 32345,
            "range": "± 170",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1406,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1602,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3194,
            "range": "± 217",
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
          "id": "ab451d28c080138b8a92a578bd0eb38fb7751ae2",
          "message": "fix: close partition forks and expand consistency evidence (#364)\n\n## Description\n\nThis aggregate change repairs current-main dependency breakage, advances\nthe active consistency-evidence P0, and fixes the split-brain defect\nfound by the new matrix.\n\n- Restore `parking_lot` 0.12 after #359 followed a Loom path dependency\nand changed the root manifest to the Rust-1.95-only 0.13 line without\nsynchronizing all locks. Apply every root MSRV hold to every auxiliary\nCargo updater, with a drift-proof regression test.\n- Supersede stale install-action 2.89.0 update #362 with immutable\nv2.90.1 pins at every use, including the composite installer.\n- Add an expectation-aware partition ledger at N=2/3/4/8/16. It\npreserves complete oracle diagnostics and replay evidence, checks\npre-cut progress and every directed injection, classifies bounded\nrecovery/safe failure/defect/invalid evidence, and runs matched\ncorruption controls.\n- Fix witnessless automatic `ContinueWithout` timeouts so two-endpoint\npartitions cannot commit opposite one-member certificates. Explicit\n`remove_player` and remote Goodbye retain their existing one-survivor\nbehavior; uncertified `Disconnected` events stay suppressed, and\nspectator same-poll Goodbye handling stays consistent.\n- Enforce the complete oracle verdict in the existing healthy cost sweep\nwithout changing the sweep-v3 JSON schema. Publish both matrices through\nnightly artifacts and document exact evidence at `8c8660b`.\n\nThe active plan remains open for reorder, duplication, bursts,\nasymmetry, repeated flaps, poll stalls, exact timing/resource\nmeasurements, and component-scoped majority/minority state evidence.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No new `unwrap()` in production code\n  - No new `expect()` in production code\n  - No new `panic!()` or `todo!()`\n  - New production errors use existing explicit state/error paths\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [x] I have updated relevant examples in the `examples/` directory (no\nAPI signatures or example behavior required changes)\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- Witnessless two-endpoint and multi-handle automatic-timeout\nregressions.\n- Explicit local removal and receiver-side Goodbye compatibility\ncoverage.\n- Spectator same-poll input-plus-Goodbye regression coverage.\n- Hot-join witnessless-timeout terminal-state coverage; obsolete\nauto-drop/rejoin assumptions removed.\n- Dependabot root/child MSRV-rule drift regression.\n- Partition schedule, classifier, replay, premise, terminal-plateau, and\nmatched negative-control coverage.\n\n**Validation performed:**\n\n- `cargo nextest run --no-capture`: **2,981 passed, 73 skipped**.\n- `cargo nextest run --features hot-join --no-fail-fast --no-capture`:\n**3,239 passed, 74 skipped**.\n- Exact nightly release matrices at\n`8c8660bf37341eff246bf85d7913d2e2c6ce0c18`: **2 passed**; 136 cost rows\nplus 24 partition rows.\n- Partition ledger: **5 bounded recoveries, 15 safe failures, 4\nintentional defect controls**; zero replay mismatches, zero premise\nviolations, four typed controls, and a 64-step plateau in every\nsafe-failure row.\n- Exact PR gates: **2 passed**; ten baseline rows match the checked-in\ncost ledger with zero desyncs.\n- `cargo +1.86.0 check --locked --all-targets`: **passed**.\n- `python3 scripts/release/workspace_locks.py check`: **all four\nworkspaces passed**.\n- `cargo update --dry-run`: **zero compatible updates** in every Cargo\nworkspace; only intentional MSRV-held root releases remain outdated.\n- `.venv/bin/pytest scripts/tests -q`: **2,186 passed**.\n- `cargo test --doc -- --nocapture`: **160 passed, 50 ignored**.\n- Rustdoc with `-D warnings`, Actionlint, Markdownlint, plan hygiene,\nchangelog policy, wiki consistency, link checking, doc claims, and\nbranch-explicit agent preflight: **passed**.\n- Final adversarial review after Goodbye and spectator fixes: **zero\nfindings**.\n\n## Related Issues\n\n- Supersedes #362 with the newer install-action release and the required\nmain-lock repair.\n- Advances the active P0 in `PLAN.md`; no open GitHub issues existed\nwhen this work began.",
          "timestamp": "2026-11-05T10:44:08-08:00",
          "tree_id": "41ce73583aee3306fd54af4d46ee938429c83570",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ab451d28c080138b8a92a578bd0eb38fb7751ae2"
        },
        "date": 1793904785291,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60433,
            "range": "± 639",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 30863,
            "range": "± 359",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1785,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 114",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2995,
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
          "distinct": true,
          "id": "1e2d8e2825c733a8fe1b2fe48076193621e4a0ce",
          "message": "test: add duplication evidence and repair verification driver (#367)\n\n## Description\n\nAdvances the active consistency-evidence P0 with deterministic\ndirected-duplication rows and closes verification-tooling gaps found\nduring the issue/dependency review.\n\n- Add per-directed-link duplication counters and exact target-only\ninjection premises.\n- Add duplication recovery rows at N=2/3/4/8/16 plus matched in-window\nN=2/N=16 state-corruption controls.\n- Include the new counters in complete replay comparison and document\nexact-SHA results for the 31-row matrix.\n- Make `verify-all.sh` run the implemented Z3 suite in default and\nparallel modes, propagate Z3 failures, and restore parallel TLA+/Kani\nresults from background subshells.\n- Correct verification documentation about Z3 model scope, fixed\nqueue-size assumptions, negative frames, and the existing\nruntime-to-TLA+ handshake trace refinement.\n- Refresh compatible Cargo lockfiles in all four workspaces; remaining\ndirect upgrades are intentionally MSRV-held.\n\n`PLAN.md` remains active locally with only duplication removed from the\noutstanding axes. No public API, wire format, production Rust behavior,\nor changelog-visible behavior changes.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change\n- [x] 📚 Documentation\n- [ ] ♻️ Refactor\n- [x] 🧪 Test\n- [x] 🔧 CI/Build (verification tooling and dependency locks)\n\n## Checklist\n\n### Required\n\n- [x] I have read the contributing guide and repository engineering\npolicy.\n- [x] I followed the zero-panic policy; no production Rust behavior\nchanged.\n- [x] I added regression and matrix coverage for the changed behavior.\n- [x] `cargo fmt && cargo clippy --workspace --all-targets --features\ntokio,json` passed.\n- [x] `cargo nextest run --no-capture` passed: **2,986 passed, 73\nskipped**.\n\n### If Applicable\n\n- [x] Documentation and synchronized wiki pages updated.\n- [x] CHANGELOG reviewed: no entry needed for\ntest/tooling/docs/compatible-lock changes.\n- [x] No new compiler warnings.\n\n## Testing\n\n**Tests added/modified:**\n\n- Directed SimNet duplication and saturating counter-merge tests.\n- Declared schedule/injection premises and complete replay checks for 31\npartition/duplication rows.\n- Five fake-tool regressions for default/parallel Z3 execution, Z3\nfailure propagation, and parallel TLA+/Kani failure propagation.\n- Z3 documentation/comment alignment; no proof expectations weakened.\n\n**Validation performed:**\n\n- Exact-SHA full release matrix at\n`fbbe1d5b3d097623cbdff3a454a0f6b33c93ee1d`: **31 rows**, comprising **10\nbounded recoveries, 15 safe failures, and six intentional defect\ncontrols**. Every row SHA matched; replay mismatches and premise\nviolations were both zero.\n- Five duplication bases injected 174 duplicate sends each, only on 0→1,\nwith per-link sums equal to the global counter. N=2/N=16 in-window state\ncontrols produced typed peer/frame evidence.\n- `cargo test --test verification --features z3-verification-bundled --\nz3 --nocapture`: **65 passed**.\n- `pytest scripts/tests -q`: **2,191 passed**.\n- `cargo test --doc -- --nocapture`: **160 passed, 50 ignored**.\n- Rustdoc with `-D warnings`, Ruff, shell portability, workspace-lock\nchecks, Cargo MSRV check, links (1,444), doc claims, wiki consistency,\nMarkdownlint, plan hygiene, and branch-explicit agent preflight:\n**passed**.\n- `cargo update --dry-run`: zero remaining compatible updates. Latest\n`main` at `ab451d2`: **all 15 push workflows succeeded**.\n\n## Related Issues\n\n- #365 was investigated and closed separately with its bounded\nLean/Aeneas feasibility result.\n- Advances the active P0 consistency-evidence matrix; reorder, bursts,\nasymmetry, repeated flaps, poll stalls, exact timing, and host resource\nevidence remain tracked there.",
          "timestamp": "2026-11-05T17:19:02-08:00",
          "tree_id": "6875ac9f4756ffd772b51399b24841761da34f1a",
          "url": "https://github.com/wallstop/fortress-rollback/commit/1e2d8e2825c733a8fe1b2fe48076193621e4a0ce"
        },
        "date": 1793928444012,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 25457,
            "range": "± 624",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 15228,
            "range": "± 320",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 455,
            "range": "± 11",
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
            "value": 5491,
            "range": "± 700",
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
          "id": "cab3078685cb9f237a4229512c99aa27f47416f5",
          "message": "test: add deterministic reorder consistency evidence (#368)\n\n## Description\n\nAdvances the active P0 consistency-evidence matrix with deterministic\nsame-link reorder coverage and refreshes the browser-test dependency\nclosure.\n\n- Add per-directed-link and mesh-global reordered-delivery counters,\nincluding saturating merge and complete replay evidence.\n- Add directed jitter/reorder windows at N=2/3/4/8/16, with exact\ntarget-only injection premises and matched N=2/N=16 in-window\nstate-corruption controls.\n- Independently validate reorder telemetry against actual receiver\npayload order.\n- Bump the partition ledger to schema v3 and document exact-SHA results\nfor the 38-row matrix.\n- Update `wasm-bindgen-test` to 0.3.80 and the matching wasm-bindgen\nCLI/lock closure to 0.2.130.\n\n`PLAN.md` remains active locally with only reordering removed from the\noutstanding axes. No public API, wire format, production Rust behavior,\nor changelog-visible behavior changes.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change\n- [x] 📚 Documentation\n- [ ] ♻️ Refactor\n- [x] 🧪 Test\n- [x] 🔧 CI/Build (test dependency and runner refresh)\n\n## Checklist\n\n### Required\n\n- [x] I have read the contributing guide and repository engineering\npolicy.\n- [x] I followed the zero-panic policy; no production Rust behavior\nchanged.\n- [x] I added regression and matrix coverage for the changed behavior.\n- [x] `cargo fmt && cargo clippy --workspace --all-targets --features\ntokio,json` passed.\n- [x] `cargo nextest run --no-capture` passed: **2,991 passed, 73\nskipped**.\n\n### If Applicable\n\n- [x] Documentation and synchronized wiki pages updated.\n- [x] CHANGELOG reviewed: no entry needed for\ntest/tooling/docs/development-dependency changes.\n- [x] No new compiler warnings.\n\n## Testing\n\n**Tests added/modified:**\n\n- Directed SimNet reorder instrumentation and independent receiver-order\nvalidation.\n- Declared schedule/injection premises and complete replay checks for 38\npartition/duplication/reorder rows.\n- Five reorder recovery bases and two matched in-window state-corruption\ncontrols.\n- Workflow contract coverage for the lock-derived wasm-bindgen CLI\nversion.\n\n**Validation performed:**\n\n- Exact-SHA full release matrix at\n`2da22dc560429c45b6bfdbd9ca0d5652d23dfdbe`: **38 rows**, comprising **15\nbounded recoveries, 15 safe failures, and eight intentional defect\ncontrols**. Every row SHA matched; replay mismatches and premise\nviolations were both zero.\n- Reorder bases at N=2/3/4/8/16 each observed **105 reordered\ndeliveries**, exclusively on 0→1, with per-link sums equal to the global\ncounter. Both in-window controls produced typed peer/frame evidence.\n- `cargo nextest run --no-capture`: **2,991 passed, 73 skipped**.\n- `pytest scripts/tests -q`: **2,191 passed**.\n- `cargo test --doc -- --nocapture`: **160 passed, 50 ignored**.\n- Rustdoc with `-D warnings`, Actionlint, workspace locks, Cargo MSRV\ncheck, compatible-update dry run, links (1,445), wiki consistency,\nMarkdownlint, plan hygiene, and version sync passed locally.\n\n## Related Issues\n\n- Advances the active P0 consistency-evidence matrix; bursts, asymmetry,\nrepeated flaps, poll stalls, exact timing, and host resource evidence\nremain tracked there.\n- #365 remains the separately queued P2 source-linked Lean experiment.",
          "timestamp": "2026-11-05T20:46:11-08:00",
          "tree_id": "43d00fac19c83c18575321a0c553cdebf012bbd6",
          "url": "https://github.com/wallstop/fortress-rollback/commit/cab3078685cb9f237a4229512c99aa27f47416f5"
        },
        "date": 1793940857167,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 61118,
            "range": "± 553",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 30796,
            "range": "± 106",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1695,
            "range": "± 114",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 118",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3001,
            "range": "± 184",
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
          "id": "b960525f4c687de7522b10835f45567f95d0eb94",
          "message": "test: add deterministic burst consistency evidence (#369)\n\n## Summary\n\n- Extend the expectation-aware consistency ledger to deterministic\ndirected burst loss at N=2/3/4/8/16.\n- Add saturating per-link/global burst-start evidence,\nbackward-compatible trace identity, exact independent payload-loss span\nchecks, replay comparison, and matched N=2/N=16 corruption controls.\n- Record the schema-v4 matrix in the tuning guide/wiki and remove only\nthe completed burst axis from `PLAN.md`.\n- Upgrade the exactly pinned Playwright browser probe from 1.64.1 to\n1.65.0.\n\n## Evidence\n\nFinal code SHA `6bef90ae9b6b5165f9a1808bf38be6459886912b` passed the\nrelease-mode full matrix:\n\n- 45 rows: 35 uninjected bases and ten matched controls.\n- 20 bounded recoveries, 15 safe failures, and ten intentional defect\ncontrols.\n- Every burst row observed eight starts and 24 policy drops only on\ndirected link 0→1, with a real three-input loss run.\n- Both in-window burst controls produced typed state divergence at peer\n1/frame 120.\n- Zero replay mismatches and zero premise violations; every row carries\nthe exact code SHA.\n\nThe independent SimNet oracle reconstructs missing payload spans,\naccepts only three-packet bursts or a truncated terminal burst, and\nderives exact event counts. The matrix also permits valid coalesced runs\nand one heal-truncated final burst. Three adversarial review rounds\nclosed the initial oracle and boundary findings; the final aggregate\nreview reported zero findings.\n\n## Validation\n\n- Focused SimNet/trace/schedule tests and 13-row PR gate: passed.\n- Exact-SHA `partition_sweep_full`: 45 rows passed in release mode.\n- `cargo clippy --workspace --all-targets --features tokio,json`:\npassed.\n- `cargo nextest run --no-capture`: 2,996 passed, 73 skipped.\n- `.venv/bin/pytest scripts/tests -q`: 2,191 passed.\n- `RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps`: passed; doc tests\n160 passed, 50 ignored.\n- Link check: 1,445 links, zero errors/warnings; doc claims, wiki\nconsistency, Markdownlint, plan hygiene, typos, and four workspace-lock\nchecks passed.\n- Explicit changed-file agent preflight: all selected checks passed\nwithout fallback.\n- Playwright 1.65.0: clean `npm ci`, zero audit vulnerabilities, and all\nthree browser tests discovered.\n- Latest `main` (`cab3078`): 90 successful checks, five accepted skips,\nzero pending/failures.\n\n## Scope\n\nNo production API or wire behavior changes are included, so no changelog\nentry is needed. Asymmetry, repeated flaps, poll stalls, exact timing,\nand host resource evidence remain in the active P0 plan. The only open\nissue, #365, remains the intentionally queued P2 Lean experiment.",
          "timestamp": "2026-11-06T11:42:18-08:00",
          "tree_id": "657597e6e6e75040625f7c32b47ee79c78e5beb9",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b960525f4c687de7522b10835f45567f95d0eb94"
        },
        "date": 1793994650744,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39546,
            "range": "± 550",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24244,
            "range": "± 672",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1090,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1246,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2772,
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
          "id": "4757214205cb8b8f685df777b9c3be49993b0b85",
          "message": "test: add deterministic asymmetry consistency evidence (#370)\n\n## Summary\n\n- Extend the expectation-aware consistency ledger to a deterministic 200\nms one-way delay on link 0→1 at N=2/3/4/8/16.\n- Add saturating per-link/global per-copy delay evidence,\nbackward-compatible trace identity, exact replay comparison, and matched\nN=2/N=16 corruption controls.\n- Handle valid post-heal overtaking explicitly: clean sends may pass old\ndelayed packets, so directed reorder evidence is required rather than\nrejected.\n- Record the schema-v5 matrix in the tuning guide/wiki and remove only\nthe completed asymmetry axis from `PLAN.md`.\n\n## Evidence\n\nImplementation SHA `726b25f47fb51213c71a8ebcbb47b07dfa7757f1` passed the\nrelease-mode full matrix:\n\n- 52 rows: 40 uninjected bases and 12 matched controls.\n- 25 bounded recoveries, 15 safe failures, and 12 intentional defect\ncontrols.\n- Asymmetric delay observed 170 delayed copies and 26 post-heal reorder\ndeliveries at N=2; every larger size observed 132 delayed copies and 22\nreorder deliveries.\n- Every delay/reorder injection occurred only on 0→1, and per-link sums\nmatched global counters.\n- Both controls produced typed state divergence at peer 1/frame 120,\nbounded by the confirmed prefix at healing.\n- Zero replay mismatches and zero premise violations; every row carries\nthe exact implementation SHA.\n\nIndependent SimNet tests validate direction, exact virtual-time delivery\nboundaries, per-copy delayed duplication, and counter saturation. Four\nadversarial review rounds closed the initial oracle boundary,\ninstrumentation coverage, documentation, and progress-record findings;\nthe final aggregate review reported zero findings.\n\n## Validation\n\n- Focused SimNet/trace/schedule tests and 15-row PR gate: passed.\n- Exact-SHA `partition_sweep_full`: 52 rows passed in release mode.\n- `cargo clippy --workspace --all-targets --features tokio,json`:\npassed.\n- `cargo nextest run --no-capture`: 2,996 passed, 73 skipped.\n- `.venv/bin/pytest scripts/tests -q`: 2,191 passed.\n- `RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps`: passed; doc tests\n160 passed, 50 ignored.\n- Link check: 1,445 links, zero errors/warnings; doc claims, wiki\nconsistency, Markdownlint, plan hygiene, typos, and four workspace-lock\nchecks passed.\n- Agent preflight: all selected checks passed without fallback.\n- Latest `main` (`b960525`): all 14 push workflows successful, zero\npending/failures.\n\n## Review readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS (no production code changed)\n- Determinism: PASS (fixed seed/virtual time; complete replay equality)\n- Error handling: PASS (test-only instrumentation)\n- Test breadth: PASS (direction, timing, duplication, saturation,\nmatrix, replay, and negative controls)\n- Design log: N/A (no architecture decision)\n- CHANGELOG: N/A (no public API, production behavior, or wire change)\n\n## Scope\n\nNo production API or wire behavior changes are included, so no changelog\nentry is needed. Repeated flaps, poll stalls, exact timing, and host\nresource evidence remain in the active P0 plan. The only open issue,\n#365, remains the intentionally queued P2 Lean experiment. All\ncompatible Cargo workspace dependencies are current; remaining direct\nupgrades are held by the Rust 1.86 MSRV.",
          "timestamp": "2026-11-06T13:19:32-08:00",
          "tree_id": "40f1edc91709000424c90a38bc8d45154f049771",
          "url": "https://github.com/wallstop/fortress-rollback/commit/4757214205cb8b8f685df777b9c3be49993b0b85"
        },
        "date": 1794000428414,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39694,
            "range": "± 232",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 23084,
            "range": "± 195",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1090,
            "range": "± 0",
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
            "value": 2760,
            "range": "± 156",
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
          "id": "6fb81cb421c3016f73289838b983688598180700",
          "message": "test: complete consistency evidence matrix (#371)\n\n## Description\n\nCompletes the active P0 consistency evidence matrix and advances\n`PLAN.md` to differential rollback equivalence.\n\n- Adds repeated symmetric partition flaps and repeated local poll stalls\nat N=2/3/4/8/16.\n- Adds matched N=2/N=16 corruption controls and validates every fault\npremise.\n- Records per-heal recovery, exact stall freeze/resume windows,\ncontiguous confirmation-stall durations, and host elapsed/RSS evidence.\n- Publishes the schema-v6 results in the tuning guide/wiki and records\nthe session evidence.\n\nNo production or public API code changes.\n\n## Type of Change\n\n- [x] 📚 Documentation (changes to documentation only)\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy** (no production code\nchanged)\n- [x] I have added tests that prove the new evidence and controls work\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] My changes generate no new compiler warnings\n\nCHANGELOG and examples are not applicable because this is test\ninstrumentation and documentation only.\n\n## Testing\n\n**Tests added/modified:**\n\n- Expanded `partition_sweep_pr_gate` from 15 to 19 rows.\n- Expanded `partition_sweep_full` from 52 to 66 rows, each with a\ncomplete deterministic replay.\n- Added actual per-stall and per-heal observations to the simulation\nharness.\n\n**Verification performed:**\n\n- Exact-SHA (`ba287019533c01734b8f3a96430812db918a633a`) release full\nmatrix: 66 rows passed; 35 bounded recoveries, 15 safe failures, 16\nintentional defect controls; zero replay mismatches and zero premise\nviolations.\n- Full `cargo nextest run --no-capture`: 2,996 passed, 73 skipped.\n- `cargo clippy --workspace --all-targets --features tokio,json`: passed\nwith no warnings.\n- Doc tests: 160 passed, 50 ignored; rustdoc with warnings denied\npassed.\n- Link, semantic-doc, wiki, Markdown, plan-hygiene, typos,\nworkspace-lock, security, and agent-preflight checks passed.\n- Latest `main` SHA `4757214205cb8b8f685df777b9c3be49993b0b85`: all 14\npush workflows successful.\n\n## Related Issues\n\nLeaves the P2 Lean pilot #365 in its existing priority order.",
          "timestamp": "2026-11-06T20:03:55-08:00",
          "tree_id": "2a5ef287bf4f3995be2b8da5b37b646b7b2200b9",
          "url": "https://github.com/wallstop/fortress-rollback/commit/6fb81cb421c3016f73289838b983688598180700"
        },
        "date": 1794024726408,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 43928,
            "range": "± 448",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25798,
            "range": "± 390",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 678,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 871,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 6839,
            "range": "± 306",
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
          "id": "a6615805df71fb845de580e55a30a1b19ea10c08",
          "message": "test(simulation): add differential rollback reference (#372)\n\n## Description\n\nAdvances the active P0 rollback-equivalence work with an independent\nfixed-history oracle and refreshes all compatible Cargo lockfile\npatches.\n\nThe simulation harness previously proved that peers agreed with one\nanother, which could not detect a shared wrong input or state.\nDifferential mode now compares typed confirmed inputs and exact\n`StateStub` trajectories against a nominal non-networked reference. The\nbounded matrix covers EveryFrame/Sparse, static input delay 0/2,\ndelayed/held/reordered delivery, and forward/reverse/rotating peer drive\norder.\n\nThe change also adds replayable mutation controls for shared input\nerrors, common-mode state errors, missing confirmed states, padding-only\ntyped input changes, and checksum-only detector lies. Legacy runs retain\ntheir pre-change trace identity, pinned against base SHA\n`6fb81cb421c3016f73289838b983688598180700`.\n\nNo production or public API code changes. Runtime membership/delay\nchanges, spectators, hot-join, rebinding, near-overflow frames, and\narbitrary game-state semantics remain explicitly bounded in the active\nP0 plan item.\n\n## Type of Change\n\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (dependency lockfile refresh)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy** (no production code\nchanged)\n- [x] I have added tests that prove the new reference and mutation\ncontrols work\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the progress documentation accordingly\n- [x] My changes generate no new compiler warnings\n\nCHANGELOG and examples are not applicable because this is test\ninfrastructure and compatible lockfile maintenance only.\n\n## Testing\n\n**Tests added/modified:**\n\n- Added 13 differential-oracle tests covering the green matrix, exact\nreplay, cross-order confirmed equivalence, mutation controls, option\nvalidation, artifact round trips, legacy trace preservation, and\nshrinking.\n- Extended artifact schema v8 and corpus validation for differential\noptions and reference failures.\n- Added explicit missing-state and invalid wide-input artifact rejection\ncoverage.\n\n**Verification performed:**\n\n- Full Nextest: 3,009 passed, 73 skipped.\n- Full simulation target: 379 passed, 26 skipped.\n- Strict all-workspace/all-target Clippy with `tokio,json`: passed\nwithout warnings.\n- Agent preflight: all selected checks passed without fallback.\n- Link checking: 1,442 links, zero errors or warnings; semantic\ndocumentation and typo checks passed.\n- Independent adversarial design and implementation reviews completed;\nfinal re-review reported zero remaining issues.\n- `cargo audit`, `cargo deny check`, all four workspace lock checks, npm\naudit/outdated checks, strict latest-docs build, and 200\nrelease-automation tests passed.\n- Latest `main` SHA `6fb81cb421c3016f73289838b983688598180700`: 90\nsuccessful checks, five intentional skips, zero failures or pending\nchecks.\n\n## Related Issues\n\nLeaves the P2 Lean pilot #365 in its existing priority order. No other\nopen issues or PRs existed at reconciliation.",
          "timestamp": "2026-11-07T11:52:57-08:00",
          "tree_id": "29d4c82d2abe48ce706fc5875af34a2ef02dc8cd",
          "url": "https://github.com/wallstop/fortress-rollback/commit/a6615805df71fb845de580e55a30a1b19ea10c08"
        },
        "date": 1794081670574,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60384,
            "range": "± 676",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 29974,
            "range": "± 480",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1861,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
            "range": "± 90",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3133,
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
          "id": "b799615d3008e55a901ad288a183cdf9b616b705",
          "message": "test(simulation): prove runtime delay equivalence (#373)\n\n## Description\n\nAdvances the active P0 rollback-equivalence work by extending the\nindependent differential reference to one successful, strict,\npost-synchronization runtime input-delay increase.\n\nThe oracle now records the actual `(peer, current frame, old delay, new\ndelay)` control boundary and independently models replicated gap inputs\nand resumed ordinals. The bounded matrix covers EveryFrame/Sparse,\ndelayed/held/reordered delivery, and forward/reverse/rotating peer drive\norder, with additional nonzero initial-delay coverage.\n\nThe change also hardens final-state validation so a bad game-state frame\ncannot lower its own validation ceiling, preserves legacy shrinker\nbehavior, and records the work in\n`progress/session-191-runtime-delay-equivalence.md`.\n\nNo production or public API code changes. Membership changes,\nspectators, hot-join, broader operation interleavings, and arbitrary\ngame-state semantics remain explicitly bounded in the active P0 plan\nitem.\n\n## Type of Change\n\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the contributing guide and repository development\npolicy\n- [x] I have followed the zero-panic policy (no production code changed)\n- [x] I have added tests that prove the new reference and mutation\ncontrols work\n- [x] I have run strict workspace Clippy with no warnings\n- [x] I have run the full Nextest suite and all tests pass\n\n### If Applicable\n\n- [x] I have updated the progress documentation accordingly\n- [x] My changes generate no new compiler warnings\n\nCHANGELOG and examples are not applicable because this is test\ninfrastructure only.\n\n## Testing\n\n**Tests added/modified:**\n\n- Hard-coded 0→3 and 2→4 gap/resume known-answer vectors.\n- Full save/delivery/drive-order runtime-delay matrix with exact replay\nand matched no-change controls.\n- Changed-player input mutation, shared-state, checksum-lie, final-frame\nmismatch, artifact replay, validation, and shrinker controls.\n- Independent final-state ceilings and legacy non-differential delay\nshrinking regressions.\n\n**Red-green evidence:**\n\n- Before the nominal gap model,\n`runtime_delay_increase_matches_nominal_gap_and_resume` failed at frame\n80 because the actual stream repeated ordinal 79 while the static\nreference advanced.\n- Final focused differential/shrinker gate: 25 passed.\n- Full simulation target: 391 passed, 26 ignored.\n- Full Nextest: 3,021 passed, 73 skipped.\n- Strict all-workspace/all-target Clippy with `tokio,json`: passed\nwithout warnings.\n- Rustdoc with warnings denied, plan hygiene, diff checks, and agent\npreflight passed; link checking covered 1,442 links with zero errors or\nwarnings.\n- Independent adversarial review found two issues; both were fixed with\nregressions, and final re-review reported zero remaining issues.\n\n**External reconciliation:**\n\n- Latest `main` SHA `a6615805df71fb845de580e55a30a1b19ea10c08`: 89\nsuccessful checks, five intentional skips, zero pending or failed\nchecks.\n- No compatible Cargo or npm updates remained. Cargo audit/deny and all\nfour workspace lock checks passed.\n\n## Related Issues\n\nLeaves the P2 Lean pilot #365 in its existing priority order. No other\nopen issues or PRs existed at reconciliation.",
          "timestamp": "2026-11-07T13:41:40-08:00",
          "tree_id": "7b688bdfe96a57e7468f74e88be112b06a6a40e2",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b799615d3008e55a901ad288a183cdf9b616b705"
        },
        "date": 1794088170618,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39018,
            "range": "± 514",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24848,
            "range": "± 87",
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2765,
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
          "id": "4c102a9aacb16dfe98175fd675f41012ac56b23d",
          "message": "test(simulation): prove spectator reference equivalence (#374)\n\n## What\n\nExtends the bounded two-peer differential rollback oracle to spectator\nsessions. Every displayed spectator frame is now checked against the\nindependent fixed-history input model, including input fingerprints,\n`Confirmed` statuses, and missing-frame detection. Normalized spectator\nhistory participates in exact replay and cross-drive-order comparisons.\n\n## Why\n\nPeer/spectator agreement alone cannot detect a shared wrong input\nstream. This change advances the active P0 rollback-equivalence work in\n`PLAN.md` by giving the spectator path an independent oracle and\nexplicit mutation controls.\n\n## Coverage\n\n- EveryFrame and Sparse saves\n- Delayed, held, and reordered delivery\n- Forward, reverse, and rotating peer drive orders\n- Single and redundant spectator hosts\n- Runtime input-delay increase from 2 to 4, including the complete\ngap/resume boundary prefix\n- Shared wrong-input and spectator-only corruption negative controls\n- Exact trace and normalized-trajectory replay\n\nThe claim remains bounded to the existing two-peer Lockstep scenarios.\nRuntime membership changes, hot-join, rebinding, near-overflow frames,\nand arbitrary game-state semantics remain active follow-up scope in\n`PLAN.md`.\n\n## Validation\n\n- Focused spectator tests: 3 passed\n- Full simulation target: 394 passed, 26 skipped\n- Full Nextest: 3,024 passed, 73 skipped\n- `cargo clippy --workspace --all-targets --features tokio,json`: passed\nwithout warnings\n- `cargo doc --no-deps`: passed\n- Rustdoc tests: 160 passed, 50 ignored\n- Agent preflight, formatting, plan hygiene, typos, semantic doc claims,\nand 1,442 documentation links: passed\n- `cargo audit`, `cargo deny check`, `npm audit`, and all four workspace\nlock checks: passed\n- Compatible dependency updates: none; all three standalone Cargo\nworkspaces and npm are current\n\nNo production or public API code changed, so no changelog entry is\nrequired.\n\n## Review readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS (no production diff)\n- Determinism: PASS (seeded schedules and ordered `BTreeMap` histories)\n- Error handling: PASS (existing structured oracle failures)\n- Test breadth: PASS (nominal matrix, replay, boundaries, and negative\ncontrols)\n- Design log reviewed: N/A (test-only oracle extension)\n- Changelog reviewed: N/A (no user-observable/public change)",
          "timestamp": "2026-11-07T20:26:33-08:00",
          "tree_id": "f0dec225f030c4f991d4c8a50562bd75b8eb6035",
          "url": "https://github.com/wallstop/fortress-rollback/commit/4c102a9aacb16dfe98175fd675f41012ac56b23d"
        },
        "date": 1794112407679,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 25526,
            "range": "± 2797",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 15511,
            "range": "± 282",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 471,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 588,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 5393,
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
          "id": "1af6062c20f76662830344655c12a8f714a6de84",
          "message": "test(simulation): prove graceful removal equivalence (#375)\n\n## What\n\nExtends the bounded two-peer differential rollback oracle to one\nexplicit graceful removal. The independent reference now models the\ncertified freeze cut, including exact input and `StateStub` boundaries,\nrather than relying only on peer agreement.\n\nAlso keeps session evidence in the local, gitignored progress log and\nnarrows the active P0 item in `PLAN.md` to the remaining membership\nscenarios. Per review, all previously tracked progress logs are removed\nfrom git and canonical agent guidance now explicitly keeps them\nmachine-local.\n\n## Why\n\nA mesh can agree on the same wrong frozen input or state. This change\nadvances the active rollback-equivalence milestone by checking the\nsurviving fixed-membership trajectory against an independent nominal\nhistory and by proving that abandoned pre-removal observations cannot\ncreate false divergence.\n\n## Reference semantics\n\nFor certified cut `F`:\n\n- Input `F` remains nominal; frames after `F` repeat that value for the\nremoved target.\n- Post-state `F+1` consumes nominal input `F` and remains\nboundary-valid.\n- Post-state `F+2` is the first state that can observe frozen input.\n- The departed peer's required `F+1` state remains checked when\nconfirmed and simulated; its abandoned `F+2+` generation is excluded\nfrom reference and legacy cross-peer state comparisons.\n- Target-authored confirmed inputs above `F` cannot define the\nsurvivor's frozen continuation.\n\nThe two-peer commit guard requires synchronous disconnection, a Running\nsurvivor, a cut equal to the pre-call direct receipt, and pre-call\nsurvivor confirmed/sampled history bounded by the cut.\n\n## Coverage and controls\n\n- EveryFrame/delay-0/reordered and Sparse/delay-2/delayed schedules\n- Forward, reverse, and rotating drive orders\n- Exact trace, trajectory, control, and failure-vector artifact replay\n- Matched no-removal input/state controls and post-removal\nrollback/load/resimulation evidence\n- Frozen-input, wrong-state, missing-state, and final-frame mutation\ncontrols\n- Explicit non-vacuity evidence for target-authored inputs above `F` and\nabandoned target states at `F+2+`\n\nThe claim remains bounded to two peers and Lockstep cadence. Automatic\ndisconnects/timeouts, spectator membership changes, hot-join, and the\nother active P0 scenarios remain in `PLAN.md`.\n\n## External reconciliation\n\n- Only open issue: #365, intentionally P2 behind the active correctness\nwork\n- No pre-existing open, draft, or dependency PRs\n- No compatible Cargo or npm updates; fuzz, Loom, and Godot standalone\nworkspaces are current\n- Main CodeQL push cancellation was caused by replacement with the\nscheduled run at the same SHA; that scheduled run completed\nsuccessfully, so no source/workflow fix was needed\n\n## Validation\n\n- Focused differential selection: 32 passed\n- Full simulation target: 399 passed, 26 skipped\n- Full Nextest: 3,029 passed, 73 skipped\n- `cargo clippy --workspace --all-targets --features tokio,json`: passed\nwithout warnings\n- Rustdoc: passed; documentation tests: 160 passed, 50 ignored\n- Agent preflight, formatting, diff checks, plan hygiene, typos,\nsemantic doc claims, and 1,442 documentation links: passed\n- All four workspace lock checks, `cargo audit`, `cargo deny check`, and\n`npm audit`: passed\n- Independent adversarial review iterated through all findings; final\nre-review reported zero issues\n\nNo production or public API code changed, so no changelog entry is\nrequired.\n\n## Review readiness\n\n- Build/tests: PASS\n- Zero-panic: PASS (no production diff)\n- Determinism: PASS (seeded schedules and ordered histories)\n- Error handling: PASS (structured oracle failures and explicit commit\nguard)\n- Test breadth: PASS (nominal matrix, exact replay, boundaries, and\nnegative controls)\n- Agent preflight: PASS\n- Design log reviewed: N/A (test-only oracle extension)\n- Changelog reviewed: N/A (no user-observable/public change)",
          "timestamp": "2026-11-07T22:55:19-08:00",
          "tree_id": "1560cc841c7610a370172436b9a3217ba5c286c2",
          "url": "https://github.com/wallstop/fortress-rollback/commit/1af6062c20f76662830344655c12a8f714a6de84"
        },
        "date": 1794121401979,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60480,
            "range": "± 1323",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 30940,
            "range": "± 1068",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1863,
            "range": "± 113",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3141,
            "range": "± 205",
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
        "date": 1794254242762,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 25956,
            "range": "± 673",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 17311,
            "range": "± 1286",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 489,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 607,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 5744,
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
          "id": "ede612d6a1988fad6f60dedbbc633e4799e4251b",
          "message": "test(simulation): extend differential oracle to N-peer membership (#378)\n\n## Description\n\nAdvances the active P0 differential rollback-equivalence milestone\n(#377) by widening the bounded fixed-history oracle to N-peer\nmembership, spectator host failover, and queue-capacity boundaries.\n\n### Changes\n\n- **N-peer hot-join (n = 2 and 3).** The harness now models production's\nasynchronous all-survivor removal certificate: the replacement stays\noffline, the old generation is frozen out of the drive loop, the\ncertified cut is anchored when the coordinator observes the commit, and\nreference sampling is suspended until the cut is known. The independent\nreference is width-aware: a two-peer reactivation rewrites snapshot `A`,\nwhile an N-peer serve keeps `A` as a frozen bridge and activates at `A +\n1`. Activation validation now covers every survivor, and exact\ncoordinator/survivor event payloads are pinned.\n- **Three-peer explicit removal + redundant spectators.**\n`GracefulRemove` now supports asynchronous certificates on three-peer\nmeshes. Spectator rows cover host sets `[0]`, `[2]`, `[0, 2]`, and `[2,\n0]`, validating exact frozen fingerprints/statuses, host survival, exact\nreplay, and the certified cross-order projection. The removed player's\nown endpoint is explicitly rejected as a spectator source because\nproduction does not withdraw it and its stale stream correctly fails\nclosed.\n- **Spectator host silence and failover.** Schedule `Block`/`Hold` may\ntarget the spectator endpoint, isolating one host's observer feed.\nPost-removal rows silence the canonical and lower-priority hosts with a\ntimeout above the notify delay and below the outage length, proving the\nsurviving host advances the display, the blocked endpoint is observed as\nfailed, and the exact frozen history resumes with replay identity.\n- **Input-queue capacity boundaries.** New rows fill the queue from the\ndelay side (`delay 120 + prediction 7`) and the prediction side (`delay\n2 + prediction 125`, `MAX_FRAME_DELAY = 127`) and retain the exact\nnominal confirmed history past the boundary.\n- **Cross-order identity.** The bounded hot-join row's activation frame\nis proven order-stable and the complete normalized trajectory is\nidentical across forward/reverse/rotating drive orders.\n- `PLAN.md` is narrowed to the remaining\nhost-crash-as-second-membership-change row and further queue/ring\ncorners.\n\n## Type of Change\n\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy** (no production code\nchanged)\n- [x] I have added tests that prove the coverage is effective\n- [x] `cargo fmt` and strict Clippy (`--workspace --all-targets\n--features tokio,json` and `...,hot-join`) pass with no warnings\n- [x] `cargo nextest run` passes\n\n### If Applicable\n\n- [x] `PLAN.md` updated (test-only change, so no changelog entry)\n\n## Testing\n\n- Differential suites: 37 default and 41 with `hot-join` at the first\ncommit; 38/42 after the host-failover addition, all passing.\n- Full Nextest: 3,036 default and 3,298 with `hot-join`, 73/74\nintentionally skipped.\n- Strict Clippy with `-D warnings` under both feature sets, `cargo fmt`,\nagent preflight, plan hygiene, doc links, typos, and markdownlint pass.\n- Red-green evidence: the N-peer removal initially failed the\nsynchronous-commit premise, then exposed the frozen bridge-frame\nboundary; both were fixed at the harness/reference level and restored\ngreen without production changes. The host-outage row initially failed\nthe builder's notify-delay bound and was corrected to a 600 ms timeout\nabove the 500 ms notify delay.\n\n## Related Issues\n\nRelates to #377. Remaining scope tracked there: a host crash that is\nitself a second membership change, the removed player's own spectator\nendpoint (documented fail-closed), and further queue/ring corners.",
          "timestamp": "2026-11-09T15:23:39-08:00",
          "tree_id": "5eea0a5444b70ce360011f33b9913fd3613744d1",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ede612d6a1988fad6f60dedbbc633e4799e4251b"
        },
        "date": 1794267135244,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60483,
            "range": "± 5685",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 30054,
            "range": "± 939",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1689,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 115",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3144,
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
          "distinct": true,
          "id": "6050a99cf07296ca14f5a2318d66b0175c040925",
          "message": "test: warm allocation control before measuring exact sensitivity (#379)\n\n## Description\n\nRCA for the `Build & Test (macos-latest)` failure on main (`ede612d`,\njob 129511543039):\n\n```\nthread 'warmed_hot_paths_obey_allocation_contracts' panicked at tests/allocation_contract.rs:447:\nassertion `left == right` failed: known-allocation control: Stats { allocations: 6, ..., bytes_allocated: 5052, ... }\n  left: 6\n right: 1\n```\n\n`stats_alloc` materializes its per-thread accounting lazily on first\nuse, so the first measured region on that macOS runner was charged six\none-time allocations (956 bytes beyond the known 4 KiB). The same code\npassed on the PR's macOS job minutes earlier, and the\nallocation-contract binary is independent of the simulation-harness\nchanges in that merge.\n\nThe fix charges that one-time setup to a discarded warm-up region before\nthe exact sensitivity control. The control remains exact (one\nallocation, 4,096 bytes, zero reallocations) on every platform, and all\nzero-allocation assertions keep their full teeth.\n\n## Type of Change\n\n- [x] 🧪 Test (adding or updating tests)\n\n## Checklist\n\n- [x] `cargo fmt` and strict Clippy pass\n- [x] `cargo nextest run -E\n'test(warmed_hot_paths_obey_allocation_contracts)'` passes\n- [x] No production code changed; no changelog entry required\n\n## Testing\n\n- `warmed_hot_paths_obey_allocation_contracts` passes locally.\n- Strict Clippy (`--workspace --all-targets --features tokio,json`)\npasses with `-D warnings`.\n- macOS CI in this PR validates the runner-specific RCA.",
          "timestamp": "2026-11-09T16:01:11-08:00",
          "tree_id": "7c51e6c1d9fbb92bbf9713bfa6e9a5bf2fe2f311",
          "url": "https://github.com/wallstop/fortress-rollback/commit/6050a99cf07296ca14f5a2318d66b0175c040925"
        },
        "date": 1794269337912,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39564,
            "range": "± 72",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 23709,
            "range": "± 803",
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
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2765,
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
          "id": "582180484bc7a95c7c17c7c482c9c1721df45c61",
          "message": "fix(replay): settle recorded checksums after rollback corrections; close #377 membership closure (#381)\n\n## Description\n\nTwo changes, aggregated into one PR:\n\n1. **Fix a production replay-recording bug (#380).**\n`P2PSession::advance_frame`\nrecorded confirmed-input rows and their saved-state checksums before the\ncorrection `LoadGameState`/`SaveGameState` requests it had just queued\nwere\nexecuted by the application. Every frame recorded in a call that\nrepaired a\nmisprediction therefore captured its pre-correction speculative\nchecksum, so\n   decoding and playing that recording through a fresh session reported\n`ReplayDesync` at exactly the corrected frames even though the live\nhistory\n   was correct.\n2. **Close the #377 differential membership scope.** Model a second\ncertified\n   membership cut (a player host crash composed with the first explicit\nremoval), pin the removed player's non-withdrawn spectator endpoint\ncontract\n(certified history when a survivor is canonical, fail-closed when the\nremoved\n   endpoint is canonical), and add a long-run saved-state ring-wrap row.\n\n`PLAN.md`: the P0 differential rollback-equivalence milestone is\ncomplete; the\npartition/membership CAP matrix is promoted to the active P0 item.\n\n### Fix details\n\n- `pending_replay_checksum_refresh` tracks the lowest frame whose\nrecorded\nchecksum may be speculative; `adjust_gamestate` arms it when recording\nis\n  enabled.\n- `finalize_pending_replay_checksums` re-reads that range from the\nsaved-state\nring at the next `advance_frame` (after the application executed the\nprevious\ncall's requests) and before `take_replay` / `into_replay`. A frame whose\ncell\nis no longer buffered records no checksum rather than a value the\nsession has\n  since replaced.\n- `ReplayRecorder::set_checksum` provides the index-addressed overwrite.\n- Red evidence (pre-fix): 13 rollbacks recorded frames 5..11\nspeculatively and\nplayback reported `ReplayDesync` at frames 6,7,8,9,10,11. Green: the\nsame\nwitness replays cleanly with all 19 checksums present, and a positive\ncontrol\n  (one corrupted checksum) still reports a desync.\n\n### Test details\n\n- **Host crash composed with removal:** the reference carries two\nindependent\ncertified cuts; the bounded four-peer row (remove slot 1, then kill\nplayer\nhost 2 or 3 under `ContinueWithout`) proves survivor convergence past\nthe\nsecond cut, spectator failover with no divergence, cross-order\ncertified-prefix\nidentity, replay determinism, and a post-cut wrong-state mutation\ndetection.\n- **Removed player's spectator endpoint:** surviving canonical host\nkeeps the\n  exact certified frozen history; removed canonical host fails closed\n  (`SpectatorProgressMissing`) without forking.\n- **Ring wrap:** 480 steps at `max_prediction = 1` (two-cell ring), both\nsave\n  policies, all drive orders.\n\n### Dependency refresh\n\n`cargo update`: `itoa` 1.0.18 → 1.0.19, `syn` 3.0.9 → 3.0.10. Remaining\noutdated roots are deliberately pinned majors/minors.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `tests/sessions/replay_recording.rs` (new): records a two-peer session\nthat\n  repairs a burst of mispredictions and replays it through\n`ReplaySession::new_with_validation` for both `EveryFrame` and `Sparse`.\n- `tests/simulation/differential.rs`: `host_crash_after_removal_*`,\n\n`removed_player_spectator_endpoint_keeps_certified_history_or_fails_closed`,\n\n`saved_state_ring_wrap_with_minimal_prediction_window_matches_nominal_history`,\n  plus the composed-membership validation contract.\n\n**Manual testing performed:**\n\n- `cargo fmt` / `cargo clippy --workspace --all-targets --features\ntokio,json` /\n  `--all-features` → clean\n- `cargo nextest run --no-capture` → 3042/3042 passed, 73 skipped\n- `python3 scripts/ci/agent-preflight.py --auto-fix --files ...` → all\nchecks passed\n- `python3 scripts/hooks/check-plan-hygiene.py` → pass\n\n## Related Issues\n\nCloses #380\nCloses #377\n\n\n### Update: first #382 CAP-matrix row\n\n`tests/simulation/harness/components.rs` adds a declared component\nlayout\n(`RunOptions::component_scope`) and a component-scoped comparison\n(`classify_component_scope`) that distinguishes a failed-closed\npartition from a\none-sided split match and from two components exposing different values\nfor the\nsame confirmed frame (a fork). The first bounded row\n(`symmetric_partition_components_fail_closed_without_fork`) drives a\npermanent\n2/2 partition over four peers under `ContinueWithout`: both components\nstop at\nthe same cut, all peers end `Synchronizing`, no `Disconnected` event\nappears, the\nreviewed terminal failure set holds with no input/state divergence, and\na\nmutation control that rewrites both members of one component on the last\nshared\nframe flips the classification to `Forked`.\n\nMeasured note recorded on #382: a 3-peer singleton isolation (1/2 split)\nalso\nfails closed on every peer today, so no majority-continues component\nexists yet.\n\n\n### Update: CAP row 2 (timeout-versus-slow-peer) and #365 item 1\n\n- **CAP row 2:** a three-peer mesh where peer 2's application pauses for\n960 ms\n(below the 2 s timeout) must keep one confirmed history: classification\n`ContinuedTogether`, final confirmed 833/834/835, all peers `Running`,\nzero\n`Disconnected`/`PeerDropped`/`DesyncDetected`. The matched negative\ncontrol\npauses the same peer for 3.2 s: it is certified as dropped, the\nsurvivors move\n  on, and the branch is reported as `Forked` with an explicit\n`ConfirmedInputDivergence` — detected, never silent. `ComponentOutcome`\nnow\ndistinguishes `ContinuedTogether` from `SplitMatch`, and the component\ncut is\ncaptured at the first declared `PeerStall` as well as at a\ncross-component\n  `Block`.\n- **#365 ordered work item 1:**\n`.agents/skills/kani/references/kani-obligations.md`\n  records the exact `Frame::buffer_index` and varint contracts/domains/\n  assumptions/exclusions. Three new Tier-1 proofs pass locally:\n  `proof_frame_buffer_index_invalid_none` (2.3 s),\n  `proof_frame_buffer_index_valid_modulo` (2.4 s), and\n`proof_varint_full_u64_roundtrip_fixed_buffer` (6.3 s, every `u64`,\nten-byte\nbuffer). A symbolic 64-bit divisor exceeded 7 minutes (15-minute timeout\non the\nunbounded form), so arbitrary-size exactness is recorded as a Lean\ncandidate\n  instead of a claimed Kani result.\n\n\n### Update: CAP rows 3-4 and #365 ordered work item 2\n\n- **Row 3 (majority continuation):** a four-peer mesh with `PeerKill {\npeer: 3 }`\nkeeps survivors 0/1/2 `Running` and confirming to 573/573/574 while the\nkilled\n  peer stops at 114. Classification `SplitMatch { shared_prefix: 114,\ncontinued: [0] }`, no oracle failures, and a mutation control that flips\nit to\n  `Forked`.\n- **Row 4 (concurrent disjoint graceful removals):** two removals at the\nsame\nstep (0→1, 2→3) fail closed on every component at the shared cut with\nonly the\nreviewed drop-timeout/`EndProgress` diagnostics and zero\n`DesyncDetected` or\ninput/state divergence — an explicit terminal outcome, never a silent\nfork.\n- **#365 item 2 (extraction smoke test, pass):** `cargo-hax 0.4.2` +\nAeneas\n`build-183e4f0` + hax-lean `v0.3.30` + Lean 4.31.0 extract the unchanged\n`Frame::buffer_index` into Lean; `lake build` → `Build completed\nsuccessfully\n(1735 jobs)`. Recorded in the obligations reference with the exact pins\nand\n  the tuple-struct/derive `Unreachable` finding.\n\n\n### Update: #365 ordered work item 3 (production varint seam)\n\n- The private LEB128 implementation moved to the dependency-free\n`src/rle/varint.rs` with its own `VarintError`; `src/rle.rs` maps it\nonto the\nexact `FortressError` payloads. Differential tests compare decode\nagainst a\n  pre-refactor reference for every encoded prefix and offset, malformed\n  continuations, short-buffer prefix writes, and boundary roundtrips\n  (full suite 3056/3056).\n- The unchanged production file extracts through cargo-hax/Aeneas\n(pinned\n`build-183e4f0`, hax-lean `v0.3.30`) and the generated Lean package\nbuilds\n(1735 jobs). `encoded_len_zero` is proven; the loop-heavy contracts are\n  recorded as deferred with their full-domain Kani/Rust evidence.\n- Compiling-source mutation control: `encoded_len(0) → 2` still extracts\nbut\n  falsifies `encoded_len_zero`; the restored source verifies green.\n\n\n### Update: #365 item 4 (hardening/measurement/decision) and CAP row 5\n\n- **`lean-varint-pilot` harness:**\n`scripts/verification/lean-varint-pilot/`\ndeterministically re-extracts the unchanged production varint source,\nrejects\nstale/pin-drifted output through a normalized `MANIFEST.sha256`, builds\nthe\npackage, and audits axioms/proof holes. Measured: warm extraction ~1.1\ns,\nwarm verify 1.2 s ×3, cold project verify 118–136 s, dependency cache\n7.8 GB\n  raw / 2.8 GB gzip. Decision recorded: **not retained** — only\n`encoded_len_zero` is Lean-proven (Kani already covers the full\ncontract),\nloop-heavy proofs need `partial_fixpoint` induction, and no owner exists\nfor\n  the pinned stack.\n- **CAP row 5 (coordinator loss):** a four-peer mesh where the\ngraceful-removal\ncoordinator is killed before inventory commits fails closed on the\nsurvivors\nat the shared cut (113/114, `Synchronizing`), with zero `DesyncDetected`\nand\n  no input/state divergence; a mutation control flips the comparison to\n  `Forked`.\n\n\n### Update: heal/reflap probe found #383, and #365 is closed\n\n- A deterministic heal-after-partition probe found a post-heal split\n(peers 1\nand 3 resume on divergent histories at frames 96–103, no\n`DesyncDetected`).\nFiled as #383 with the exact schedule/evidence and tracked in `PLAN.md`.\nThe\ncomponent classifier now compares post-floor tails, which classifies the\nprobe\n  as `Forked { frame: 96, … }` instead of the previous `FailedClosed`.\n- #365 is closed with the rejected-experiment decision recorded: all\nfour\nordered items are complete, but the Lean lane does not yet prove a\nproperty\n  beyond Kani, the loop proofs are deferred, and no owner exists.",
          "timestamp": "2026-11-10T09:24:39-08:00",
          "tree_id": "d595075c0551670b68bd904c34c83e53322ab70a",
          "url": "https://github.com/wallstop/fortress-rollback/commit/582180484bc7a95c7c17c7c482c9c1721df45c61"
        },
        "date": 1794332072889,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 60359,
            "range": "± 617",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 31254,
            "range": "± 126",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1668,
            "range": "± 116",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1557,
            "range": "± 112",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3001,
            "range": "± 245",
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
        "date": 1794355648432,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39235,
            "range": "± 2704",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 17505,
            "range": "± 165",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 535,
            "range": "± 71",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 720,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 5522,
            "range": "± 294",
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
        "date": 1794363430138,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 43588,
            "range": "± 330",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 26063,
            "range": "± 483",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 675,
            "range": "± 75",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 870,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 6828,
            "range": "± 251",
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
        "date": 1794370062938,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 49872,
            "range": "± 304",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 31266,
            "range": "± 502",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1407,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1602,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 3219,
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
          "id": "a40e0edae66924b127c77c7ceeca519420bb9dfb",
          "message": "feat(transport): expose adapter-level socket metrics (#391)\n\n## Summary\n\nStarts the next queued PLAN item by making adapter-level transport work\nobservable. Previously\n`PeerMetrics` reported protocol enqueue demand only: callers could not\ndistinguish accepted sends\nfrom adapter drops, raw ingress from decoded messages, or a receive-poll\ncap saturation from an idle\npoll.\n\n- New public `SocketMetrics` with `receive_polls`, `datagrams_received`,\n`bytes_received`,\n`messages_decoded`, `datagrams_rejected`, `receive_cap_saturations`,\n`datagrams_sent`,\n  `bytes_sent`, and `send_failures`.\n- `SessionMetrics::socket` carries the live adapter snapshot for\n`P2PSession` and\n`SpectatorSession`; replay/sync-test sessions keep the default zero\nvalue.\n- `NonBlockingSocket::socket_metrics` is a provided trait method\nreturning zeroed counters, so\nexisting custom adapters compile unchanged. Built-in UDP and Tokio\nadapters override it; the\n  chaos wrapper delegates to its inner adapter.\n- `receive_all_messages_from` records raw datagrams/bytes, decoded\nmessages, decode rejections, and\nraw/decoded per-poll cap saturation without changing the existing\n256-attempt / 256-message\n  bounds.\n- Docs, changelog, public API snapshot, and PLAN in-progress item\nupdated.\n\n### Why\n\nThis closes the \"record actual socket throughput/loss, not only enqueued\nbytes\" and \"overload is\nvisibly refused or degraded\" parts of the transport P1 evidence: rising\n`socket.send_failures`\nidentifies adapter drops, and rising `socket.receive_cap_saturations`\nidentifies sustained ingress\nsaturation. A successful UDP send remains best-effort and is documented\nas adapter acceptance, not\nproof of delivery.\n\n## Tests\n\n- New real-loopback UDP test: accepted send/bytes counters, decoded\nreceive counters, then a raw\nmalformed datagram increments `datagrams_rejected` without changing\ndecoded counts or cap\n  saturation.\n- Expanded deterministic receive-cap test: 256 malformed datagrams\nreport 256 raw receives, 256\nrejects, 0 decoded, and exactly one cap saturation; the valid datagram\nbehind them decodes on the\n  next poll.\n- New session test pins the adapter→`SessionMetrics::socket` merge path.\n- `cargo fmt --all --check`\n- `cargo clippy --workspace --all-targets --features tokio,json -- -D\nwarnings`\n- `cargo nextest run --features tokio,json`: 3,109 passed, 73 skipped, 0\nfailed\n- `python3 scripts/ci/agent-preflight.py --auto-fix`: all checks passed\n- `python3 scripts/api/public_api_census.py --check`: 2,742 symbols\nmatch\n- `markdownlint`, `typos`, changelog/version/plan hooks: clean\n- Diff mutation gate: adapter `socket_metrics` overrides and\n`record_send_failure` are caught; the two\nsemantically-equivalent default-trait delegates and mutually-exclusive\nchaos wrappers are excluded\n  in `.cargo/mutants.toml` with rationale\n\nTransport P1 remains in progress: retransmission storms, browser\nsuspension, a real-transport\nthroughput/loss sampler, and downstream queue-age evidence still remain.",
          "timestamp": "2026-11-10T22:32:49-08:00",
          "tree_id": "6ef79a19821fa77167b4def144ffd3274b949345",
          "url": "https://github.com/wallstop/fortress-rollback/commit/a40e0edae66924b127c77c7ceeca519420bb9dfb"
        },
        "date": 1794379269512,
        "tool": "cargo",
        "benches": [
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 39544,
            "range": "± 634",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25597,
            "range": "± 287",
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncLayer/256_frame_save_advance",
            "value": 2778,
            "range": "± 205",
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
      }
    ]
  }
}