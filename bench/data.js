window.BENCHMARK_DATA = {
  "lastUpdate": 1784175806376,
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
          "id": "87196ccbfb60f3863723b179d49480a2f8c05e06",
          "message": "feat: Fix Multiple Distributed Systems Problems on Multi-Peer Sessions for Connect/Reconnect (#182)\n\n## Description\n\nThis PR closes critical gaps in multi-peer disconnect/drop convergence,\nadds a sequence-numbered floor-round to prevent divergence in relay\ntopologies, and hardens codec/protocol handling against stale signals.\nThe biggest practical impact is in sessions with 3+ peers.\n\n### User-Facing Changes\n\n- **Fixes divergence after peer drop**: Survivors no longer confirm past\nan agreed freeze point in staggered-detection and relay scenarios.\n- **Closes double-failure-relay corner**: Introduces\n`FloorRequest`/`FloorReply` messages to establish a pessimistic per-slot\nfloor from relay peers, preventing silent divergence when an origin\nsurvivor dies mid-relay.\n- **Wire-format change**: All peers in a session must run a compatible\nbuild. Mixed old/new builds cannot exchange the new floor-round\nmessages.\n- **Prevents mesh agreement stalls**: Idle endpoints now emit periodic\nconnect-status nudges while a drop awaits mesh agreement, fixing a\ndeadlock at 3+ players.\n- **Expanded TLA+ validation**: Multiple new specifications and\nconfigurations validate safety properties under double-failure relay,\nasync ack, and reorder scenarios.\n\n### Technical Details\n\n- Prediction queue now always begins at the first missing frame, fixing\nsilent divergence when re-simulation requests land above missing\nwindows.\n- Connect-status gossip now carries the fold of local-received and\nremote-gossiped views, matching GGPO semantics for freeze barriers.\n- Codec and message decoding hardened against denial-of-service from\nhostile length prefixes.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `tests/sessions/peer_drop.rs` (peer-drop convergence scenarios)\n\n**Manual testing performed:**\n\n- TLA+ spec/config scenario expansion under\n`specs/tla/DoubleFailureRelay*`\n- (No additional manual runtime testing documented in this draft)\n\n## Related Issues\n\n- (None linked)\n\n---\n\n<!-- CURSOR_SUMMARY -->\n> [!NOTE]\n> **Medium Risk**\n> Changes are mostly docs, TLA+, and CI, but they encode a wire-format\nand API narrative (floor-round vs legacy gossip) that must stay aligned\nwith production; mis-synced docs or TLA pairing could hide real protocol\ndrift.\n> \n> **Overview**\n> Aligns **user-facing and internal docs** with the S55 floor-round\ndesign: **CHANGELOG** now describes the double-failure-relay fix as\nclosed by **`FloorRequest`/`FloorReply`** (replacing per-`Input`\npessimistic-floor gossip), and **doc-code-sync** points canonical\nreferences at **`FloorReply::floors`**, **`UdpProtocol::round_floor`**,\nand **`P2PSession::pessimistic_floors`**.\n> \n> Adds **CI guards** so removed identifiers (`peer_pessimistic_floor`,\n`Input::pessimistic_floor`) cannot reappear in tracked docs as current\nproduction without historical qualifiers (`check-doc-claims.sh` +\ntests). **TLA FIX_MODE consistency** is generalized:\n**`check-tla-config-consistency.py`** discovers every spec with an\n`ASSUME FIX_MODE` set, pairs `.cfg` files by filename stem, and\nvalidates README counts against any spec’s mode count (not only\n`DoubleFailureRelay`).\n> \n> **Formal verification** expands materially:\n**`DoubleFailureRelay.tla`** gains the S52 **`REORDER`** corner and\nthree new modes (**`AsyncAckSoundEpoch`**, **`AsyncAckSoundRound`**,\n**`AsyncAckSoundRoundSeq`**) with matching demo/live/witness `.cfg`\nfiles; **`SpectatorReactivationEpoch`** is registered in\n**`verify-tla.sh`** with a default `.cfg` and a large\n**`specs/tla/README.md`** write-up. Existing DoubleFailureRelay configs\npick up **`CONSTANT REORDER = FALSE`** for backward-compatible runs.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nffb856bf1e4989b971ed1f2c11caf09eacad4ccd. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-06-20T16:09:57-07:00",
          "tree_id": "79e7a6ecb02e914ab9104d5bb9ce2c1934f13256",
          "url": "https://github.com/wallstop/fortress-rollback/commit/87196ccbfb60f3863723b179d49480a2f8c05e06"
        },
        "date": 1781997264561,
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
            "value": 108,
            "range": "± 1",
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
            "value": 448,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 696,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1014,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 132447,
            "range": "± 1020",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45601,
            "range": "± 426",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1557,
            "range": "± 96",
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
          "id": "f9d3b6b636f486e75f65ec026f7d0fd36e362c67",
          "message": "bugfix: Stability in 3+ Player Sessions + Docs Overhaul (#183)\n\n## Description\n\nThis PR improves stability and convergence in 3+ player sessions,\nespecially during peer drops and relay-heavy topologies.\n\nUser-facing highlights:\n\n- Fixes silent divergence after staggered peer-drop detection by\nenforcing a safer shared freeze barrier.\n- Adds a floor-round (`FloorRequest` / `FloorReply`) so survivors do not\nconfirm past relay-agreed freeze floors.\n- Prevents mesh-agreement stalls by sending periodic connect-status\nnudges while waiting for drop agreement.\n- Fixes prediction-window behavior so rollback starts at the first\nmissing frame, avoiding missed resimulation windows.\n- Restores per-session violation observer routing across `P2PSession`,\n`SyncTestSession`, and `SpectatorSession`.\n- Expands formal verification coverage with new TLA+ n-peer\nfreeze-convergence specs/configs.\n- Updates user and architecture docs to match the new behavior.\n\nCompatibility note:\n\n- Protocol wire format now includes new floor-round messages, so peers\nin the same live session must run compatible builds.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [x] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `tests/sessions/peer_drop.rs` (multi-peer drop convergence and relay\nscenarios)\n\n**Manual testing performed:**\n\n- Added and checked TLA+ n-peer serve/freeze convergence models under\n`specs/tla/NPeerServeFreezeConvergence*`.\n- Verified spec registration updates in `specs/tla/README.md` and script\ncoverage.\n\n## Related Issues\n\n- (None linked)\n\n---\n\n<!-- CURSOR_SUMMARY -->\n> [!NOTE]\n> **High Risk**\n> Changes touch core rollback networking (confirmed-frame barrier, wire\nprotocol, input-queue prediction) where bugs cause silent desync or\nsession stalls; violation routing is lower risk but spans all session\nentry points.\n> \n> **Overview**\n> **3+ player stability** tightens how survivors agree on freeze floors\nand confirmed frames after staggered drops: `confirmed_frame()` follows\nmesh gossip (with connect-status nudges when idle), prediction episodes\nstart at the **first missing** input frame (avoiding silent divergence),\nand the protocol adds **`FloorRequest` / `FloorReply`** for relay floor\nrounds — **peers in one session need compatible builds**.\n> \n> **Violation observers** now match the documented API:\n`push_violation_observer` / `ScopedObserverGuard` make\n`report_violation!` use a thread-local stack; P2P, sync-test, and\nspectator sessions install the configured observer on public entry\npoints (including construction), with tests that fail if scoping is\nremoved.\n> \n> **Verification & tooling:** new **`NPeerServeFreezeConvergence.tla`**\n(and GateBlind/Witness cfgs) models the hot-join serve gate vs\nper-survivor freeze convergence; `verify-tla.sh` registers it;\nintegration tests gain **`create_filtered_channel_mesh`** for N≥5\nasymmetric loss; docs/specs/changelog/README align with hot-join,\nepochs, and structured errors; minor CI/doc fixes (lychee URL pattern,\nshared `CARGO_TARGET_DIR` in markdown-code tests).\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n49406793792a777a3e3a16306d399a32790b4fb5. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-06-21T19:37:19-07:00",
          "tree_id": "cd5478a0078233f09ddad5dfc2f748893c978087",
          "url": "https://github.com/wallstop/fortress-rollback/commit/f9d3b6b636f486e75f65ec026f7d0fd36e362c67"
        },
        "date": 1782096137363,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 165,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 465,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 735,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1082,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 114080,
            "range": "± 5451",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 47751,
            "range": "± 277",
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
            "value": 1601,
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
          "id": "505d631f9357be3964f3ae24b076043957c7bac0",
          "message": "Doc Upgrades + Release Target Fixes (#184)\n\n## Description\n\nThis PR finalizes the `0.9.0` release-targeted docs and release tooling\nupdates.\n\nUser-facing highlights:\n\n- Refreshes project positioning text across README/docs/wiki\n(correctness-first messaging, formal-verification emphasis, and updated\nsupport/contact guidance).\n- Consolidates and promotes unreleased changelog content into a dated\n`0.9.0` section so release notes are emitted from the right section.\n- Fixes release automation so publish now also stamps the changelog date\nand syncs bug-report version options on the default branch.\n- Adds guardrail tests to prevent future regressions in changelog\nconsolidation, issue-template version wiring, sync-version stamping\nbehavior, and unaffiliated community links.\n\nCompatibility note:\n\n- No runtime/protocol behavior changes are introduced in this branch.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `scripts/tests/test_changelog_release_consolidation.py`\n- `scripts/tests/test_issue_template_versions_wiring.py`\n- `scripts/tests/test_no_unaffiliated_links.py`\n- `scripts/tests/test_sync_version.py` (new `--stamp-release-date`\ncoverage)\n\n**Manual testing performed:**\n\n- Verified release-workflow wiring updates in\n`.github/workflows/publish.yml` and\n`.github/workflows/sync-issue-template.yml`.\n- Verified changelog/version consistency updates in `CHANGELOG.md` and\n`scripts/sync-version.sh`.\n\n## Related Issues\n\n- Addresses release-note and issue-template sync regressions (issues\n#167 and #168 references in tests/comments).\n\n---\n\n<!-- CURSOR_SUMMARY -->\n> [!NOTE]\n> **Low Risk**\n> Changes are documentation, changelog, CI/config, and release scripts\nonly; no production Rust networking or protocol code is modified.\n> \n> **Overview**\n> **Release and changelog:** In-flight `0.9.0` content is moved out of\n`## [Unreleased]` into a dated `## [0.9.0]` section (placeholder date\nfor dev; real date stamped at publish). Several `### Fixed` items drop\nthe `**Pre-existing:**` prefix.\n> \n> **Publish automation:** After `cargo publish` and the GitHub release,\n`publish.yml` now commits to the default branch (with retries): stamps\nthe changelog header via `sync-version.sh --stamp-release-date\n--release-version`, and refreshes the bug-report version dropdown via\n`sync-issue-template-versions.py --ensure-version`—because token-created\nreleases do not fire the standalone `release` workflow.\n> \n> **Tooling & CI:** `sync-version.sh` gains release stamping and fails\non unresolved metadata; legacy `pre-commit` refuses version bumps with a\ndirty tree; Cargo registry settings (`http.multiplexing = false`,\n`net.retry = 10`) and narrower `.cargo/config.toml` path filters are\nadded across workflows; semver failure messaging distinguishes API\nbreaks from crates.io flakes.\n> \n> **Docs & guardrails:** README, `lib.rs`, and docs/wiki shift to\ncorrectness-first positioning, drop the GGPO Discord link, and point\nsupport to GitHub Issues. New tests cover changelog consolidation,\npublish wiring, sync stamping, and unaffiliated links.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\na573327cd38cb66610a1e1f7efcd4709276d1406. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-06-22T14:37:57-07:00",
          "tree_id": "19091f35b85839ed1d5f098ce22ef49c3a75b0fc",
          "url": "https://github.com/wallstop/fortress-rollback/commit/505d631f9357be3964f3ae24b076043957c7bac0"
        },
        "date": 1782164545194,
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
            "value": 119,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 165,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 472,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 727,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1089,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 118527,
            "range": "± 258",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 47890,
            "range": "± 433",
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
            "range": "± 3",
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
          "id": "70938662471d747b299b203a22437f060398ab18",
          "message": "Hardening: DST simulation fleet (M0/M1) + wire-exact bandwidth accounting (D1) (#189)\n\n## Summary\n\nDistributed-systems hardening for the rollback netcode, in two coherent\nparts.\n\n### M0/M1 — deterministic whole-mesh simulation fleet (commit `b3c1b67`)\n- **M0 (D6):** ping/RTT round-trip timing virtualized onto the\ninjectable protocol clock (`ping_epoch_base`/`ping_millis`) —\nquality-report RTT can no longer be corrupted by wall-clock steps\n(NTP/VM restore) and is exactly reproducible under a virtual clock.\n`js-sys` dropped as a `wasm32` dependency.\n- **M1:** a seeded virtual-time whole-mesh simulation harness — `SimNet`\nmessage switch (per-link drop/dup/delay/jitter/partition/hold), a\nstoryline schedule generator, and an invariant oracle (confirmed-prefix\nagreement, end-state agreement, in-band desync consistency, liveness),\neach with a negative control; PR smoke fleet of 8 seeds × N∈{2,3,4}.\n- **Real bug found & fixed by the fleet on its first 4-player run\n(D8):** a permanent whole-mesh confirmation deadlock at N≥3.\nConnect-status gossip rides only `Input` packets; once every peer\nexhausted its prediction window with acked send queues, stale\n`Frame::NULL` caches never refreshed and the mesh froze permanently on a\nhealed network with every session `Running`. Fixed by arming the\nconnect-status nudge on the deadlock signature (window exhausted AND\ngossip-bound below receipts), with reserved hot-join endpoints excluded.\n\n### M2 §5.1 — wire-exact bandwidth accounting (D1, commit `6ace99e`)\n- `NetworkStats::kbps_sent` accounted `std::mem::size_of_val(&Message)`\n— the constant in-memory enum size, identical for every packet\nregardless of payload — so reported send bandwidth was fiction. Sends\nare now metered with a new alloc-free arithmetic\n`Message::encoded_len()` that is byte-exact against the codec.\n- Regression: proptest `encoded_len == codec::encode(&msg).len()` for\narbitrary messages of every variant (hot-join under `cfg`); plus a test\ndocumenting the `size_of_val` divergence.\n\n## Validation\n- `cargo fmt` + `cargo clippy --workspace --all-targets` (`tokio,json`\nand `…,hot-join`) — clean.\n- `cargo nextest run` — 2336 passed (default) / 2574 passed (hot-join),\n55 skipped.\n- `python3 scripts/ci/agent-preflight.py` — all checks pass.\n\nChangelog carries `**Pre-existing:** ### Fixed` entries for D6, D8, and\nD1.\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **High Risk**\n> Changes P2P confirmation liveness logic and alters reported\nbandwidth/ping semantics; incorrect nudge timing could affect production\nmesh behavior or hot-join.\n> \n> **Overview**\n> Fixes **network telemetry** and a **multi-player liveness deadlock**,\nand adds a **deterministic whole-mesh simulation** test stack.\n> \n> **Bandwidth and ping:** Outbound metering now uses alloc-free\n`Message::encoded_len()` (proptest vs `codec::encode`) instead of\n`size_of_val(&Message)`. `kbps_sent` is corrected to kilobits/sec (×8,\n÷1000). Quality-report RTT uses the injectable protocol clock via\n`ping_epoch_base`/`ping_millis`, not wall time; **`js-sys` is removed**\non `wasm32`. Send counters use **`u64`** to avoid wrap on 32-bit\ntargets.\n> \n> **N≥3 confirmation deadlock (D8):** Connect-status gossip only rides\non `Input`. When every peer exhausted its prediction window with\nfully-acked queues, stale gossip could freeze confirmation forever. The\nconnect-status nudge now also arms when the prediction window is\nexhausted **and** mesh gossip binds confirmation below local receipts\n(`gossip_holds_confirmation_below_receipts`); reserved hot-join\nendpoints are never nudged.\n> \n> **Simulation fleet (M0/M1):** New `SimNet` (seeded per-link faults,\nvirtual clock), schedule generator, harness oracle, PR smoke tests for\n2–4 players, pinned regression for seed 1 / 4 players, integration tests\nfor deterministic ping, and hidden `diagnostic_connect_status` for stall\ndebugging. `serde_json` gains **`float_roundtrip`** for schedule corpus\nartifacts.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n822e944145b55daf2e1ee14bbf60b48eb776723a. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\n---------\n\nCo-authored-by: Claude Opus 4.8 (1M context) <noreply@anthropic.com>",
          "timestamp": "2026-07-04T11:21:34-07:00",
          "tree_id": "a289c07fbaf20a6b573655fcec0ac043b561e713",
          "url": "https://github.com/wallstop/fortress-rollback/commit/70938662471d747b299b203a22437f060398ab18"
        },
        "date": 1783189595748,
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
            "value": 111,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 160,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 459,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 716,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1028,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 126051,
            "range": "± 2462",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45147,
            "range": "± 146",
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
          "id": "56db89d456c26258ea01499db2e5837fdc2a96d3",
          "message": "Hardening M2 §5.4: SessionMetrics + D9 event-discard telemetry (#190)\n\n## Summary\n\nAdds an always-on, pull-based **session metrics layer**\n(`src/metrics.rs`) and uses its first counter surface to fix defect\n**D9**: event-queue overflow no longer discards undrained events\nsilently.\n\nThis is the next increment of the M2 milestone (metrics + baseline\nsweep) after M2 §5.1 (D1, #189). It ships the metrics-module foundation\nplus the event-queue-overflow accounting that closes D9's telemetry gap.\n`PeerMetrics`, the rollback/stall/confirmation-lag counters, and the\nbaseline sweep are follow-up PRs — every field lands with a consuming\nsite so nothing is dead code under `deny(warnings)`, and\n`SessionMetrics` is `#[non_exhaustive]` so those are additive.\n\n## The defect (D9)\n\nBoth session types bounded the event queue with `while event_queue.len()\n> max { pop_front() }` — dropping the **oldest** event with **zero**\ntelemetry (no violation, no counter). A slow-draining app during a churn\nburst could silently lose a safety-critical `Disconnected` or\n`DesyncDetected`. A red-doc test pinned the silence; this PR flips it\ngreen.\n\n## What's new\n\n- **`SessionMetrics`** — `#[non_exhaustive]`, `Copy`,\n`serde::Serialize`, `to_json()`/`to_json_pretty()` under `json`. First\nfields: `events_discarded_total` + `events_discarded_by_kind`\n(per-`EventKind`). Plain integers updated inline — no timers, no alloc,\nno `Instant` — so reads are deterministic and WASM-safe.\n- **`EventKind`** — a payload-free mirror of every `FortressEvent`\nvariant (`as_str()`/`ALL`/`COUNT`), plus **`FortressEvent::kind()`**.\nThe events analogue of the planned `MessageKind`.\n- **`P2PSession::metrics()` / `SpectatorSession::metrics()`** accessors.\n`trim_event_queue` now records every discarded event's kind and emits\n**one** rate-limited `Warning`/`NetworkProtocol` violation per overflow\n**episode** — re-armed on each `events()` drain, so a churn burst warns\nonce, not once per message. **Retention is unchanged** (still drops\noldest); the policy change is deferred to M4.\n\n## Red → green\n\n- Rewrote the red-doc test →\n`event_queue_overflow_records_discard_telemetry`.\n- Verified RED by temporarily neutralizing the fix (silent trim):\n`overflow must count discarded events; got 0`.\n- GREEN: the `Disconnected` canary is attributed to its kind,\n`events_discarded_total >= 1`, and a rate-limited `Warning` fires.\n- New regressions: P2P + spectator rate-limit-per-drain-gap tests,\nspectator discard test, and `metrics::tests` (index↔ALL↔as_str\nbijection, exhaustive `kind()` mapping over all variants, labeled-map\nJSON).\n\n## Validation\n\n- `cargo fmt` + `cargo clippy --workspace --all-targets` clean under\n`tokio,json` and `tokio,json,hot-join`.\n- `cargo nextest run`: **2349** passed (`tokio,json`), **2587** passed\n(`+hot-join`).\n- `RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps` clean (both feature\nsets).\n- `python3 scripts/ci/agent-preflight.py --auto-fix` — all checks pass.\n\nAn adversarial review pass verified the core (index/ALL/as_str\nbijection, `kind()` totality, custom-Serialize stability) and confirmed\nthe counter has no bypass: the only `pop_front` on the event queue is in\n`trim_event_queue`, and the only `drain(..)` is in `events()` (which\nre-arms the rate limiter).\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Touches all event-queue emission paths in P2P and spectator sessions;\nmiscounted or missed trims could still hide lost\nDisconnected/DesyncDetected events, though regressions target that\nbehavior.\n> \n> **Overview**\n> Introduces **`SessionMetrics`** (`src/metrics.rs`) with\n**`P2PSession::metrics()`** and **`SpectatorSession::metrics()`**, plus\n**`EventKind`**, **`EventKindCounts`**, and **`FortressEvent::kind()`**\nso overflow drops can be counted by category. Optional **`to_json()`** /\n**`to_json_pretty()`** when the `json` feature is enabled.\n> \n> **D9 fix:** bounded event-queue overflow still drops oldest events,\nbut each discard increments **`events_discarded_total`** and\n**`events_discarded_by_kind`**, and emits one rate-limited\n**`Warning`/`NetworkProtocol`** violation per overflow episode (re-armed\nwhen **`events()`** drains). **`handle_event`** is split so\n**`trim_event_queue`** runs after every emission path—including\n**`advance_frame`**, disconnect/hot-join, and early returns—not only\ninbound protocol handling.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n216e135095895b2b1af686d029057b136906415a. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\n---------\n\nCo-authored-by: Claude Opus 4.8 (1M context) <noreply@anthropic.com>",
          "timestamp": "2026-07-04T19:43:49-07:00",
          "tree_id": "c32a03069576a8b1d1b047b3da4b724d55778c4e",
          "url": "https://github.com/wallstop/fortress-rollback/commit/56db89d456c26258ea01499db2e5837fdc2a96d3"
        },
        "date": 1783219707035,
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
            "value": 120,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 177,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 519,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 830,
            "range": "± 68",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1174,
            "range": "± 103",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 128188,
            "range": "± 2030",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 49203,
            "range": "± 649",
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
            "range": "± 8",
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
          "id": "c48bfa296a9734c68cb13c87d1b919543ed9d7c2",
          "message": "Hardening M2 §5.2: SessionMetrics rollback + pacing counters (#191)\n\nAdds the always-on session-level rollback and pacing counters to SessionMetrics,\nwired to the P2PSession paths they measure, plus the public RollbackDepthHistogram.\n\n- New SessionMetrics fields: frames_advanced / visual_frames / resimulated_frames\n  (identity frames_advanced == visual + resimulated), rollback_count,\n  rollback_depth_histogram, max_rollback_depth, prediction_miss_count,\n  stall_count (rollback-mode only), wait_recommendations,\n  confirmation_lag_current/_max/_sum, checksums_compared/_matched/_mismatched,\n  event_queue_high_water, checksum_history_high_water.\n- Allocation-free always-on prediction-miss count; the allocating list is built\n  only when a telemetry sink is installed.\n- RunReport exposes per-peer SessionMetrics; fleet test asserts the structural\n  identities and non-zero activity across the smoke fleet with zero mismatches.\n\nReview consensus: Copilot findings (allocation, depth-0 guard) and Cursor Bugbot\nfinding (lockstep stall_count) all addressed; Bugbot re-review SUCCESS.",
          "timestamp": "2026-07-04T20:50:06-07:00",
          "tree_id": "7667aefb4830062ac7c7dc8646e00e78fd9f9fdc",
          "url": "https://github.com/wallstop/fortress-rollback/commit/c48bfa296a9734c68cb13c87d1b919543ed9d7c2"
        },
        "date": 1783223679832,
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
            "value": 120,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 166,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 494,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 771,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1116,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 127293,
            "range": "± 523",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 48462,
            "range": "± 2487",
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
            "value": 1602,
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
          "id": "18cae7cd381870418f95fb43adcc1c1fd48bb793",
          "message": "Hardening M2 §5.2: PeerMetrics per-peer wire-exact traffic + accessor (#192)\n\n## Summary\n\nM2 §5.2 continued: adds the **per-peer** metrics surface\n(`PeerMetrics`), the deferred §5.1 receive-side wire accounting, and the\n`P2PSession::peer_metrics(handle)` accessor. Every counter lands with a\nlive consuming site — no dead fields under `deny(warnings)`.\n\nBuilds on #189 (D1 wire-exact bandwidth), #190 (D9 event-discard\ntelemetry), #191 (SessionMetrics rollback/pacing counters).\n\n## What's new (public API)\n\n- **`PeerMetrics`** (`#[non_exhaustive]`, `Copy`, `Serialize`,\n`to_json`/`to_json_pretty` under `json`) — a per-peer snapshot read via\n`P2PSession::peer_metrics(handle)`:\n- **Cumulative counters:** `bytes_sent`/`bytes_received`,\n`packets_sent`/`packets_received` (wire-exact via\n`Message::encoded_len`),\n`messages_sent_by_kind`/`messages_received_by_kind`,\n`input_bytes_pre_compression`/`input_bytes_post_compression`.\n- **Instantaneous gauges:** `pending_output_len`,\n`pending_checksums_len`, `ping_ms`, `remote_frame_advantage`.\n- **`MessageKind`** — payload-free mirror of the wire message variants\n(`as_str()`/`ALL`/`COUNT`), the message analogue of `EventKind`.\n- **`MessageKindCounts`** — per-kind array counter with\n`get()`/`total()` and a self-describing labeled-map `Serialize`.\n\n## Wiring (single-site, always-on)\n\n| Counter(s) | Site |\n|---|---|\n| `messages_sent_by_kind` | `queue_message` (with the existing\n`bytes_sent`/`packets_sent`) |\n| `bytes_received`, `packets_received`, `messages_received_by_kind` |\ntop of `handle_message`, **before** any protocol-state filter |\n| `input_bytes_pre/post_compression` | `send_pending_output` compression\nsite |\n\nCounting receive before the shutdown/magic/state filters keeps\n`packets_received == messages_received_by_kind.total()` by construction\nand makes `bytes_received` a true wire-traffic meter (mirror of the send\nside). Verified there is exactly one production send site\n(`send_queue.push_back` in `queue_message`) and one production receive\nsite (`handle_message`, via the poll paths).\n\n## Design note\n\nDropped the plan's `PeerMetrics::send_queue_len`: it would collide in\nname with `NetworkStats::send_queue_len` (which actually reports\n`pending_output.len()`) while exposing only a transient internal flush\nbuffer. `pending_output_len` — the real backpressure gauge — is kept.\n\n## Tests\n\n- `metrics`: `MessageKind`/`MessageKindCounts` structural + JSON tests,\n`PeerMetrics` default/serialization.\n- `messages`: `message_body_kind_maps_every_variant` (all variants +\n`Message::kind()` delegation).\n- `protocol`: exact send byte/kind identity,\nreceive-counts-before-filters (incl. a Shutdown endpoint),\ninput-compression bytes, connection gauges.\n- `tests/network/peer_metrics.rs`: end-to-end 2-session routing\npopulates the counters via the public accessor, per-kind totals equal\npacket counters, metrics are **bit-identical across runs** (no\nwall-clock leakage), and the accessor rejects local/unknown handles.\n\n## Validation\n\n- `cargo clippy --workspace --all-targets --features\ntokio,json[,hot-join]` clean\n- `cargo nextest run --features tokio,json` → 2373 passed; `+hot-join` →\n2611 passed\n- Rustdoc `--document-private-items` clean; `cargo test --doc` 159\npassed\n- `agent-preflight.py --auto-fix` all green (changelog-unreleased,\nversion-sync, link-check, doc-claims, typos, …)\n\n## Follow-ups (M2 §5.2 remainder)\n\n- `SpectatorSession::peer_metrics()` subset and `HotJoinMetrics` (each\nwith its own consuming site).\n- §5.3 baseline sweep.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Observability-only additions on existing send/receive paths; no\nwire-format or session-behavior changes. Handle validation errors are\nexplicit for non-remote handles.\n> \n> **Overview**\n> This PR extends session metrics with a **per-peer** surface:\n**`PeerMetrics`** (via **`P2PSession::peer_metrics(handle)`**) plus\n**`MessageKind`** / **`MessageKindCounts`** for labeled traffic\nbreakdowns. Snapshots include cumulative payload bytes and packets\n(send/receive), per-kind counts, input pre/post-compression totals, and\ngauges (`pending_output_len`, `pending_checksums_len`, `ping_ms`,\n`remote_frame_advantage`). Unlike **`network_stats`**, peer metrics do\nnot require synchronization.\n> \n> Protocol endpoints now increment receive counters at the **start** of\n**`handle_message`** (before magic/shutdown filters), send-side kind\ntallies in **`queue_message`**, and compression byte totals when\nflushing **`Input`** batches. **`MessageBody::kind()`** /\n**`Message::kind()`** tie wire messages to **`MessageKind`**.\n> \n> New unit and integration tests cover kind mapping, accounting\ninvariants, and deterministic two-session **`peer_metrics`** behavior;\nthe changelog documents the public API.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n441ce79e6ac7208b3c5cdb7236ae0045efa23fcb. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\n---------\n\nCo-authored-by: Claude Opus 4.8 (1M context) <noreply@anthropic.com>",
          "timestamp": "2026-07-04T22:23:00-07:00",
          "tree_id": "c43fdb5832d37b68012c1a297a06f6de3785d588",
          "url": "https://github.com/wallstop/fortress-rollback/commit/18cae7cd381870418f95fb43adcc1c1fd48bb793"
        },
        "date": 1783229262154,
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
            "value": 111,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 161,
            "range": "± 4",
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
            "value": 708,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1048,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 126048,
            "range": "± 621",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45460,
            "range": "± 394",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1245,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1557,
            "range": "± 106",
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
          "id": "a500b62790b48aa6485489cbf14c1dbcddd1742c",
          "message": "Hardening M2 §5.2: SpectatorSession::peer_metrics(host_index) (#193)\n\n## Summary\n\nCompletes the per-peer metrics surface from PR #192 (M2 §5.2). The\nspectator's host `UdpProtocol` endpoints already accumulate wire-exact\n`PeerMetrics` counters (same endpoint type as P2P remotes/spectators) —\nthis PR adds the accessor to read them, which was flagged as a follow-up\nin PLAN.md §5.2.\n\n## Change\n\n- **`SpectatorSession::peer_metrics(host_index: usize) ->\nOption<PeerMetrics>`** — hosts addressed by dense index in\n`0..num_hosts()` (builder-priority order, matching\n`start_spectator_session_multi`).\n\n### API shape: `Option`, not `Result`\n\n`P2PSession::peer_metrics(handle)` returns `Result<_, FortressError>`\nbecause it validates an **opaque `PlayerHandle`** (structured error on a\nnon-remote handle). A spectator has **no player handles** for its\nupstream hosts — they're a dense `Vec` addressed by index with a public\n`num_hosts()` bound, so out-of-range is the *only* failure mode.\n`Option` (à la `slice::get`) is the idiomatic, non-breaking shape. A\n`Result` here would have required a new `InvalidRequestKind` variant — a\nbreaking change to a non-`#[non_exhaustive]` enum, which the plan\nreserves for M5's single atomic break.\n\n### Failover caveat (documented, and tested)\n\nHost endpoints are compacted on failover (`retain_surviving_hosts`),\nnever rebuilt. So the counters at a fixed `host_index` do **not** reset\nacross a failover — they discontinuously jump to the promoted survivor's\nalready-running totals. Per-index rate math is only meaningful between\nfailovers. This is documented and locked by a test that drives the real\n`remove_disconnected_hosts` compaction path.\n\n## Tests (3, data-driven)\n\n- `spectator_peer_metrics_out_of_range_index_is_none` — index-bound\nbehavior.\n- `spectator_peer_metrics_are_isolated_and_count_received_host_traffic`\n— multi-host session; traffic delivered to host 1 only leaves host 0 at\nzero and host 1 at exactly the delivered `packets_received` /\n`Input`-by-kind. Proves per-host isolation.\n- `spectator_peer_metrics_index_follows_surviving_host_after_compaction`\n— proves the jump-not-reset failover semantics.\n\n## Validation\n\n- `cargo clippy --workspace --all-targets --features tokio,json` —\nclean.\n- `cargo doc --no-deps --features hot-join,tokio,json,sync-send\n--document-private-items` — clean.\n- `cargo nextest run` — 2347 passed. `--features hot-join` — 2584\npassed.\n- `python3 scripts/ci/agent-preflight.py --auto-fix` — all checks pass.\n\nReviewed internally by an adversarial pass that caught (and this PR\nfixes) a rustdoc misstatement about failover counter behavior and added\nthe compaction test.\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Additive read-only API delegating to existing `UdpProtocol` metrics;\nno wire or session behavior changes beyond documentation and tests.\n> \n> **Overview**\n> Adds **`SpectatorSession::peer_metrics(host_index) ->\nOption<PeerMetrics>`** so spectators can read the same wire-exact\nper-host traffic counters that **`P2PSession::peer_metrics`** already\nexposes for remotes. Hosts are keyed by dense index in\n**`0..num_hosts()`** (builder order from\n**`start_spectator_session_multi`**); out-of-range indices return\n**`None`** instead of a **`Result`**, since spectators have no\n**`PlayerHandle`** for upstream hosts.\n> \n> Rustdoc calls out **failover semantics**: when hosts are compacted\nafter a disconnect, counters at a fixed index **jump** to the promoted\nsurvivor’s totals rather than resetting, so per-index rate math should\nbe re-anchored after **`num_hosts()`** changes. **CHANGELOG** documents\nthe new API.\n> \n> Three tests lock index bounds, per-host isolation on a multi-host\nsession, and post-compaction index behavior via\n**`remove_disconnected_hosts`**.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nb9741cb98e397709f26bc9c3e835d58e2f1ec558. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\nCo-authored-by: Claude Opus 4.8 (1M context) <noreply@anthropic.com>",
          "timestamp": "2026-07-04T23:22:43-07:00",
          "tree_id": "0aeb89b4c2a4f09e567a6804aa5713782b779717",
          "url": "https://github.com/wallstop/fortress-rollback/commit/a500b62790b48aa6485489cbf14c1dbcddd1742c"
        },
        "date": 1783232900764,
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
            "value": 111,
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
            "value": 466,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 702,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1032,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 134519,
            "range": "± 1309",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 44714,
            "range": "± 902",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1245,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 85",
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
          "id": "42492249931c0c03f52a2557e93fa304c6ffd27a",
          "message": "Hardening M2 §5.3 prep: per-peer wire-metrics aggregation in the DST harness (#194)\n\n## Summary\n\nFoundational step toward the M2 §5.3 baseline sweep, which needs a\nper-player **bandwidth ledger**. Today the simulation runner's\n`RunReport` carries only `SessionMetrics`; the wire-level counters\n(bytes/packets/by-kind) live on each session's per-remote `PeerMetrics`,\nwhich the harness consumes and drops.\n\nThis adds the aggregation and a fleet consumer — no production or\npublic-API change (test infrastructure only).\n\n## Change\n\n- **`RunReport.peer_wire: Vec<PeerWireTotals>`** — each peer's\nper-remote `PeerMetrics` folded into one per-player total: bytes/packets\nsent+received, per-`MessageKind` breakdown, and input\npre/post-compression bytes. Aggregation runs at end-of-run over the\npeer's remote handles (`PlayerHandle::new(j)` for `j != i`;\n`peer_metrics` succeeds for any remote handle regardless of sync state).\n- By-kind arrays are positional in `MessageKind::ALL` order (the\ncrate-private `MessageKind::index()` is unreachable from tests);\n`sent_by_kind` / `received_by_kind` read them by category. Instantaneous\ngauges (`pending_*`, `ping_ms`, `remote_frame_advantage`) are\ndeliberately dropped — they are not additive across links.\n- **Consumer:** `peer_wire_metrics_are_wired_across_smoke_fleet`\nasserts, per peer, the by-kind == packet-count identities (preserved by\naggregation) and that real **bidirectional** wire traffic flowed (Input\npackets each way). This exercises `P2PSession::peer_metrics` end-to-end\nunder randomized simulation — previously only covered by direct-call\nunit tests.\n\n## Validation\n\n- `cargo clippy --workspace --all-targets --features tokio,json` —\nclean.\n- `cargo nextest run -E 'test(simulation::)'` — 27 passed; `--features\nhot-join` (17-variant `MessageKind`) — 27 passed. New test green both\nfeature sets.\n- `python3 scripts/ci/agent-preflight.py --auto-fix` — all checks pass.\n\nThe `peer_wire` field is deterministic and deliberately excluded from\n`trace_hash` (it's a metric, not a correctness invariant), so\nmeta-determinism is unaffected.\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Test-only harness and fleet assertions; no production or public API\nchanges.\n> \n> **Overview**\n> Adds a **per-player bandwidth ledger** to the deterministic simulation\nharness so upcoming M2 §5.3 baseline sweeps can read wire totals from\n`RunReport` instead of dropping per-remote `PeerMetrics`.\n> \n> **`RunReport.peer_wire`** holds one `PeerWireTotals` per peer, built\nat end-of-run by summing `P2PSession::peer_metrics` across every remote\nhandle (`j != i`). Totals include bytes/packets sent and received,\nper-`MessageKind` counts (via `MessageKind::ALL` layout), and input\npre/post-compression bytes; non-additive gauges (`pending_*`, ping,\nframe advantage) are omitted.\n> \n> **`peer_wire_metrics_are_wired_across_smoke_fleet`** runs the PR smoke\nfleet (2–4 players, fixed seeds) and checks aggregation invariants\n(by-kind sums match packet counts), bidirectional traffic, Input traffic\nboth ways, and non-zero compression byte counters—exercising\n`peer_metrics` end-to-end under randomized mesh sim. **`peer_wire` is\nnot folded into `trace_hash`**, so meta-determinism tests stay\nunchanged.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n88f7e691babd3876edbcc479a69a5ea75725daf4. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\n---------\n\nCo-authored-by: Claude Opus 4.8 (1M context) <noreply@anthropic.com>\nCo-authored-by: copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>",
          "timestamp": "2026-07-05T00:05:12-07:00",
          "tree_id": "4fceb32cb4352a663f444e67c637e34574592ac9",
          "url": "https://github.com/wallstop/fortress-rollback/commit/42492249931c0c03f52a2557e93fa304c6ffd27a"
        },
        "date": 1783235390205,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 160,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 470,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 718,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1042,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 122351,
            "range": "± 220",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45386,
            "range": "± 1505",
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
            "range": "± 103",
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
          "id": "771ccee890c8dbb99041a705d269310f88f481ef",
          "message": "Hardening M2 §5.3: baseline sweep cell runner + PR gate (#195)\n\n## Summary\n\nThe M2 §5.3 baseline sweep: **deterministic, virtual-time measurement**\nof steady-state protocol cost over a controlled loss × RTT × jitter\ngrid. Where the fleet layers randomized faults to *find* bugs, the sweep\nholds each cell's link conditions constant to *measure* — per-player\nbandwidth, rollback rate/depth, confirmation lag, and pacing pressure.\n\nBuilds directly on #194's `RunReport.peer_wire` bandwidth ledger. Each\ncell is a thin wrapper over the mesh runner with a uniform per-link\n`LinkPolicy` and no fault events, so a `CellReport` is a pure function\nof `(seed, params)` — zero runner noise.\n\n## Change (`tests/simulation/baseline_sweep.rs`)\n\n- **`run_cell(CellParams) -> CellReport`** (serde-serializable):\nper-player bytes/sec (pre/post compression), messages-by-kind,\nrollbacks/100-frames, rollback-depth p50/p99/max (nearest-rank over the\nmerged histogram), confirmation-lag mean/max, stalls/min, wait-recs, and\n`desync_incidents` (the load-bearing invariant — any nonzero fails).\n- **`sweep_pr_gate`** (PR test): 5 representative cells (2p\nlan/wifi/mobile, 4p wifi, 4p loss15/rtt200) assert `desync==0`, a\nliveness floor, non-zero bandwidth, percentile ordering + `p99<=max`\ncross-check, and **bit-for-bit reproducibility** (virtual time ⇒\nidentical replay).\n- **`full_matrix_sweep`** (`#[ignore]`): the 64-cell grid at 5000\nframes; offline capture via `FORTRESS_SWEEP_OUT` JSON Lines.\n\n## Data (thresholds set from measurement, not guessed)\n\n- Gate cells: `min_final_confirmed` 328–989 → liveness floor set to 50\n(~6.5× margin).\n- **Full 64-cell matrix ran locally in 48s — every cell `desync == 0`**,\nprogress 2004–4945 at 5000 steps. Bandwidth 4–16 KB/s/player; rollback\ndepth ≤7 (bounded by `max_prediction=8`); confirmation lag capped at 8;\nstalls scale with severity. The protocol holds clean across the entire\ngrid at N=2 and N=4.\n\nAll rate divisors are NaN-guarded (including `n==0`, since `schedule()`\nbuilds the `Schedule` directly and bypasses `generate`'s `n_players>=2`\nassert), keeping the `f64` `CellReport` reflexive for the determinism\nreplay.\n\n## Deferred (documented in PLAN.md §5.3)\n\n- Input-width axis {4,32}B (harness `StubConfig` fixes a 4-byte `u32`;\nneeds a generic-input refactor).\n- Checked-in exact-value baseline (`sweep-v1.json`) as M5's cost ledger.\n- Nightly full-matrix CI job.\n\n## Validation\n\n- `cargo clippy --workspace --all-targets --features tokio,json` —\nclean.\n- `cargo nextest run -E 'test(simulation::)'` — 28 passed; `--features\nhot-join` — 28 passed.\n- `python3 scripts/ci/agent-preflight.py --auto-fix` — all checks pass.\n\nReviewed internally by an adversarial pass (confirmed the statistics,\ndeterminism, and desync signal correct; fixed a vacuous\n`p99<=max.max(BUCKETS)` assertion → real `p99<=max` cross-check, and the\n`n==0` NaN guard).\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Test-only additions with no production or harness API changes; CI\nimpact is a small fixed set of virtual-time simulation runs in\n`sweep_pr_gate`.\n> \n> **Overview**\n> Adds **M2 §5.3 baseline sweep** simulation tests: deterministic\nvirtual-time measurement of steady-state protocol cost (bandwidth,\nrollbacks, confirmation lag, stalls) over fixed loss × RTT × jitter\ncells, wired through the existing mesh harness with uniform `LinkPolicy`\nand no fault events.\n> \n> Introduces `run_cell` → serde **`CellReport`**, a PR\n**`sweep_pr_gate`** test over five representative cells\n(`desync_incidents == 0`, liveness floor, non-zero bandwidth, rollback\npercentile sanity, bit-for-bit replay), optional\n**`FORTRESS_SWEEP_OUT`** JSON Lines export, and an **`#[ignore]`** full\n64-cell matrix for offline runs. **`tests/simulation.rs`** registers the\nnew `baseline_sweep` module only.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n490bac634347b9547ada114bacef7263a0a16891. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\n---------\n\nCo-authored-by: Claude Opus 4.8 (1M context) <noreply@anthropic.com>\nCo-authored-by: copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>",
          "timestamp": "2026-07-05T00:59:39-07:00",
          "tree_id": "3eb66179b2fcb2b319feff9437f17f82a85ac3ee",
          "url": "https://github.com/wallstop/fortress-rollback/commit/771ccee890c8dbb99041a705d269310f88f481ef"
        },
        "date": 1783238645427,
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
            "value": 120,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 169,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 477,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 770,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1106,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 127608,
            "range": "± 573",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 48480,
            "range": "± 342",
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
          "id": "bbd0bd4670b410eca6aa310e0a22f02dfd69dd95",
          "message": "Hardening M2 §5.2: HotJoinMetrics — joiner hot-join handshake latency (#196)\n\n## Summary\n\nCompletes the M2 §5.2 metrics surface. Adds a `hot-join`-feature-gated\npublic `HotJoinMetrics`, read via `P2PSession::hot_join_metrics() ->\nOption<HotJoinMetrics>` (`None` for any session that did not hot-join —\na host, or a peer that synchronized normally). It reports a joiner's\nhandshake latency:\n\n- `completed` — whether the joiner reached `Running`.\n- `polls_to_running` — `poll_remote_clients` iterations spent\n`HotJoining`.\n- `millis_to_running` — elapsed time on the **injectable protocol\nclock**, so it is deterministic under the DST/simulation harness (no\nwall clock).\n\n## Design\n\n- **Clock access** reuses the session's already-stored\n`protocol_config.clock` via a `clock_now`/`now()` helper byte-identical\nto the protocol endpoint's — session- and endpoint-level timings share a\nbasis.\n- `join_started_at` is stamped at construction for a joiner (read before\n`protocol_config` is moved into the session); `became_running_at` is\nstamped by an **idempotent** `record_hot_join_activation()` (only stamps\nonce, no-op for a non-joiner).\n- **Every-path completeness (the D9 lesson):** a joiner reaches\n`Running` at three sites — 2-peer snapshot apply, N-peer snapshot apply,\nand `check_initial_sync` (reachable when a joiner is fail-closed to\n`Synchronizing` mid-handshake and later resumes without applying a\nsnapshot). All three call the shared idempotent helper, so `completed`\ncan never be permanently stuck `false` while the session is genuinely\n`Running`. **The `check_initial_sync` site was found by an internal\nadversarial review** — my first pass instrumented only the two apply\nsites; the review proved the fail-closed→resume path reaches `Running`\nuninstrumented.\n- Type matches `SessionMetrics`/`PeerMetrics` conventions:\n`#[non_exhaustive]`, `Copy`, `serde::Serialize`,\n`to_json()`/`to_json_pretty()` under `json`.\n\n## Tests\n\n`hot_join_metrics_records_joiner_latency` drives a full 2-peer join and\nasserts: host → `None`; joiner incomplete (with `millis == 0`) while\n`HotJoining`; completed with positive `polls_to_running` and\n`millis_to_running` after activation; and a stable completed metric\nacross later polls. The N-peer and fail-closed sites reuse the same\nvalidated idempotent helper (the N-peer joiner tests use a low-level\n`ManualJoiner`, not a `P2PSession`, so they can't exercise the accessor\ndirectly).\n\n## Validation\n\n- `cargo clippy --workspace --all-targets` on both `tokio,json` and\n`hot-join,tokio,json` — clean.\n- `cargo nextest run` — 2349 passed; `--features hot-join` — 2588\npassed.\n- `cargo doc --no-deps --features hot-join,tokio,json,sync-send\n--document-private-items` — clean.\n- `python3 scripts/ci/agent-preflight.py --auto-fix` — all checks pass.\n\nCompiles cleanly with and without `hot-join` (all new items\nfeature-gated; no dead code under `#![deny(warnings)]`).\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Observability-only, feature-gated instrumentation on existing hot-join\nactivation paths; no wire or handshake behavior changes.\n> \n> **Overview**\n> With the **`hot-join`** feature, this PR adds a public\n**`HotJoinMetrics`** type and **`P2PSession::hot_join_metrics() ->\nOption<HotJoinMetrics>`**. Hosts and normally synchronized peers get\n**`None`**; joiners get **`completed`**, **`polls_to_running`**\n(increments on each **`poll_remote_clients`** while **`HotJoining`**),\nand **`millis_to_running`** on the session’s **`ProtocolConfig::clock`**\n(deterministic under simulation).\n> \n> **`P2PSession`** stores joiner-side **`HotJoinTiming`**: join start at\nconstruction, activation via idempotent\n**`record_hot_join_activation()`** at all three paths to **`Running`**\n(2-peer snapshot apply, N-peer apply, and **`check_initial_sync`** after\nfail-closed resume). Optional **`to_json()`** / **`to_json_pretty()`**\nmatch other metrics types. Changelog and an integration test\n**`hot_join_metrics_records_joiner_latency`** cover the full 2-peer join\nflow.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nd877905ca6ddcf23e35c4ce96b78ad32c615adf6. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\n---------\n\nCo-authored-by: Claude Opus 4.8 (1M context) <noreply@anthropic.com>",
          "timestamp": "2026-07-05T02:16:59-07:00",
          "tree_id": "e0b0501ae18b058c998c82fc4d30e6f3aea2fc04",
          "url": "https://github.com/wallstop/fortress-rollback/commit/bbd0bd4670b410eca6aa310e0a22f02dfd69dd95"
        },
        "date": 1783243298666,
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
            "value": 120,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 167,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 472,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 768,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1072,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 127394,
            "range": "± 316",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 48492,
            "range": "± 282",
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
            "value": 1602,
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
          "id": "9a6202cbf17ec0db7c60d2fe1c3df4313d8da45f",
          "message": "Hardening M2 §5.3: checked-in sweep cost-ledger baseline + regression gate (#197)\n\n## Summary\n\nAdds the M2 §5.3 **cost ledger**: a checked-in baseline\n(`tests/simulation/baselines/sweep-v1.json`, blessed at 0.9.0 wire\nsizes) plus a regression comparison in `sweep_pr_gate`. Each gate cell's\ncost/behavior metrics are checked against the baseline within tolerance\n(plan §5.3: **bytes ±5%, rollbacks ±10%; `desync_incidents` exact 0**).\nAt M5, the +6 B/packet wire change surfaces here as a reviewed,\nover-tolerance delta — that's the ledger's purpose.\n\n## Change (`tests/simulation/baseline_sweep.rs`, test-only)\n\n- **`BaselineCell`** stores each cell's identity + measured cost\nmetrics. Volatile `version`/`git_sha` and the per-kind map are omitted\nso the JSON stays a stable, reviewable diff.\n- **`check_or_bless_baseline`**: compares the gate cells to the\nchecked-in file, or regenerates it when `FORTRESS_SWEEP_BLESS=1` is set\n(`FORTRESS_SWEEP_BLESS=1 cargo test --test simulation sweep_pr_gate`).\nThe refresh command is in the drift/missing-file panic message.\n- **`assert_close`** uses a relative + absolute-floor tolerance, so a\nnear-zero baseline (a LAN cell's tiny rollback rate) is not brittle\nunder a purely relative bound.\n\n## Why this is robust, not fragile\n\nThe metrics are a deterministic function of `(seed, params)` (virtual\ntime, integer PCG RNG, IEEE-correctly-rounded float ops), so\n**same-platform runs reproduce the baseline exactly** — the tolerances\nare pure cross-platform safety margin. This PR's **3-OS Build & Test CI\nvalidates cross-platform determinism directly** against the\nLinux-blessed file: if it passes on macOS/Windows, determinism holds;\nthe generous floors absorb any last-ULP noise. Intentional changes\nregenerate the baseline as a reviewed diff (the plan's model).\n\n## Validation\n\n- `cargo clippy --workspace --all-targets --features tokio,json` (incl.\n`-W clippy::nursery` spot-check) — clean.\n- `cargo nextest run -E 'test(simulation::)'` — 28 passed; `--features\nhot-join` — 28 passed. Gate passes against the blessed baseline; a fresh\nbless + compare round-trips exactly.\n- `python3 scripts/ci/agent-preflight.py --auto-fix` — all checks pass.\n\n## Remaining §5.3 (follow-ups)\n\nThe input-width `{4,32}B` axis (a broad generic-input harness refactor)\nand the nightly full-matrix CI job remain — see PLAN.md §5.3.\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Test-only simulation harness and checked-in JSON; no production\nruntime or auth/data paths affected.\n> \n> **Overview**\n> Adds the M2 §5.3 **cost ledger**: a checked-in baseline at\n`tests/simulation/baselines/sweep-v1.json` and a comparison step wired\ninto **`sweep_pr_gate`**.\n> \n> **`BaselineCell`** captures each gate cell’s identity and steady-state\nmetrics (bandwidth, rollbacks, lag, etc.) while omitting volatile fields\nlike `version`/`git_sha` so baseline updates stay reviewable.\n**`check_or_bless_baseline`** loads that file and compares the five PR\ngate cells: cell labels and parameters must match exactly,\n**`desync_incidents`** must stay **0**, and cost metrics use\n**`assert_close`** (bytes ±5%, rollbacks ±10%, with small absolute\nfloors for near-zero LAN rollback rates). Setting\n**`FORTRESS_SWEEP_BLESS=1`** regenerates the JSON instead of failing on\ndrift.\n> \n> This extends the existing gate (desync, liveness, determinism) with\nregression detection for wire/rollback behavior—intentional wire-format\nchanges are expected to show up as a blessed baseline diff rather than\nsilent CI pass.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n1762e9d913dba21d92b2174cbac87f21e541e8f6. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\n---------\n\nCo-authored-by: Claude Opus 4.8 (1M context) <noreply@anthropic.com>",
          "timestamp": "2026-07-05T03:10:10-07:00",
          "tree_id": "ae884b2ded053d85570aa64e134b4252baec837b",
          "url": "https://github.com/wallstop/fortress-rollback/commit/9a6202cbf17ec0db7c60d2fe1c3df4313d8da45f"
        },
        "date": 1783246476501,
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
            "value": 119,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 171,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 473,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 734,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1068,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 127349,
            "range": "± 1157",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 48796,
            "range": "± 277",
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
            "value": 1602,
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
          "id": "236e1f506ac134b72e8fb01dbf9297a77cb2cce2",
          "message": "Hardening M3 §6.1: PeerStall lifecycle fault (peer hang) in the DST harness (#198)\n\nFirst M3 §6.1 lifecycle-vocabulary fault: `ScheduleEvent::PeerStall` freezes a peer for a bounded window (local hang — GC pause / frame-time spike), distinct from the existing network-only faults. Test-infra only (no src/ or public-crate API, no changelog).\n\n- schedule.rs: new PeerStall variant; schema bump 1->2; random generator untouched so every existing seed (incl. the D8 regression) stays bit-identical.\n- harness/mod.rs: per-peer stall deadline + frozen-but-folded trace; reusable mid-run confirmed-frame probe; up-front schedule-index + probe-range validation so malformed corpus schedules fail loudly.\n- fleet.rs: freeze->recover arc (measured clean [244,245] vs frozen [195,195]); oracle-teeth-under-hitch negative control; malformed-schedule should_panic.\n\nReviewed by an adversarial sub-agent (verdict: solid) + GitHub Copilot (2 rounds, all threads resolved) + Cursor Bugbot (Low Risk).",
          "timestamp": "2026-07-05T11:14:55-07:00",
          "tree_id": "d85b1e079b7c71dcf8d23934d3cfd0a344be5948",
          "url": "https://github.com/wallstop/fortress-rollback/commit/236e1f506ac134b72e8fb01dbf9297a77cb2cce2"
        },
        "date": 1783275579302,
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
            "value": 110,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 159,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 461,
            "range": "± 13",
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
            "value": 1033,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 123352,
            "range": "± 1962",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45166,
            "range": "± 112",
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
          "id": "277a4567b012a1ec645f40d9b38e609ab2d81723",
          "message": "Hardening M3 §6.1: SetInputDelay lifecycle op (mid-run input-delay change) (#199)\n\nSecond M3 §6.1 lifecycle op: `ScheduleEvent::SetInputDelay` raises a peer's local input delay mid-run, driving `P2PSession::set_input_delay`'s mid-session gap-fill/flush reconfiguration path (replicated confirmed inputs to every remote, connect-status stamped, pending output flushed) — a path the fixed-delay fleet never exercised. Test-infra only (no src/ or public-crate API, no changelog).\n\n- schedule.rs: new SetInputDelay variant; schema bump 2->3; random generator untouched (existing seeds bit-identical).\n- harness/mod.rs: handler calls set_input_delay on the peer's local handle and surfaces any error via the oracle; peer index in the shared up-front validation.\n- fleet.rs: input_delay_increase_keeps_mesh_consistent (premise: with-vs-without diverges the trace; determinism; mesh stays byte-consistent across the reconfiguration) + oracle-teeth negative control + out-of-range should_panic. Premise decoupled from the config default via an increase delta.\n\nReviewed by an adversarial sub-agent (verdict: solid) + GitHub Copilot (2 rounds, 0 comments) + Cursor Bugbot (SUCCESS).",
          "timestamp": "2026-07-05T12:01:49-07:00",
          "tree_id": "7b07fa40098fdcad71846c8cc0936bd8c07973ad",
          "url": "https://github.com/wallstop/fortress-rollback/commit/277a4567b012a1ec645f40d9b38e609ab2d81723"
        },
        "date": 1783278392480,
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
            "value": 117,
            "range": "± 2",
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
            "value": 479,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 717,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1046,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 126170,
            "range": "± 506",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45492,
            "range": "± 513",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1245,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1557,
            "range": "± 108",
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
          "id": "99321fcbbab6a337d52a36b8dacf8d50a58f9576",
          "message": "Hardening M3 §6.1: PeerKill lifecycle op (peer crash) + oracle alive-mask (#200)\n\nThird M3 §6.1 lifecycle op — the first oracle-coupled one, introducing the alive-mask §6.2 liveness needs. `ScheduleEvent::PeerKill` crashes a peer: the harness stops driving it and detaches it from the fabric (inbox discarded, further traffic dropped), and the oracle excludes it from the *liveness* checks (end-progress + the live-peer confirmed prefix) while still comparing its pre-death states. Test-infra only.\n\n- schedule.rs: PeerKill { peer }; schema v4; round-trip covers all three lifecycle events.\n- oracle.rs: dead mask + mark_peer_dead (fail-loud on out-of-range); finalize excludes dead peers from (c-lite) and from global_confirmed, but still compares their pre-death recorded states in (b); NoLivePeers guard against an all-crashed vacuous pass.\n- harness/mod.rs: dead mask; PeerKill handler (detach + mark_peer_dead); drive loop skips dead peers; peer index in the shared up-front validation.\n- fleet.rs: peer_kill_survivors_converge_under_continue_without (GREEN — a single crash under ContinueWithout degrades gracefully: survivors stay byte-consistent, no fork) + oracle_catches_seeded_divergence_under_peer_kill (survivor corrupted post-crash — the mask lets state-agreement reach the post-crash prefix) + oracle_catches_pre_kill_divergence_on_the_killed_peer (a peer that diverged before crashing is still caught) + run_rejects_out_of_range_peer_kill.\n\nData: crashing peer 1 of 4 under ContinueWithout keeps survivors {0,2,3} byte-consistent — contrast Halt's D13 divergence and the split-brain fork.\n\nReviewed by an adversarial sub-agent (verdict: solid) + GitHub Copilot (9 rounds — 2 substantive oracle-integrity fixes: all-dead guard, pre-death divergence detection; then fail-loud + doc precision) + Cursor Bugbot (SUCCESS).",
          "timestamp": "2026-07-05T13:53:30-07:00",
          "tree_id": "6aedb1b0d9dd093120dcde374087a2d8a8a8a9cb",
          "url": "https://github.com/wallstop/fortress-rollback/commit/99321fcbbab6a337d52a36b8dacf8d50a58f9576"
        },
        "date": 1783285101793,
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
            "value": 161,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 484,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 743,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1062,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 126896,
            "range": "± 372",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 44824,
            "range": "± 438",
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
            "value": 1557,
            "range": "± 106",
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
          "id": "c52785fabc851f0f2d5e71381d1a1b561040f34b",
          "message": "Hardening M3 §6.6-pre.3: WaitRecommendation-obeying app model (H-OSC probe) (#201)\n\nCloses the time-sync control loop in the DST harness. The harness driver (like ex_game and every prior fleet run) ignored WaitRecommendation, leaving the loop open — so oscillation (H-OSC) was unobservable. Adds an AppModel { Ignore, Obey } axis: Obey skips the recommended advances (poll but do not advance, only while Running) so the ahead peer lets the others catch up. Test-infra only.\n\n- schedule.rs: AppModel enum (default Ignore, #[serde(default)] — existing seeds/corpus bit-identical, no schema bump); SimConfig.app_model; a missing-field deserialize test pins the default.\n- harness/mod.rs: per-peer wait_skip counter; under Obey, accumulate skip_frames from WaitRecommendation (max), and count it down only on Running steps that would otherwise advance.\n- fleet.rs: app_model_obey_wait_recommendation_stays_consistent (n in {2,4}) over asymmetric-delay links (symmetric emits no recommendations — the D11 bias is the source). Premise: the Obey run emits recommendations (metrics) and Obey vs Ignore diverges the trace (red-verified); determinism; both pass the oracle.\n\nH-OSC precondition data (asymmetric side-observation, NOT the symmetric FM-3 experiment, still owed): the one-sided closed loop is well-damped — obeying costs ~0-2 frames of progress over 900 steps (n=2: ignore [835,836] vs obey [833,833]) and never breaks consistency/liveness.\n\nReviewed by an adversarial sub-agent (verdict: correct, no must-fix) + GitHub Copilot (3 rounds: doc accuracy, serde-test robustness, wait_skip-only-while-Running fidelity) + Cursor Bugbot (SUCCESS).",
          "timestamp": "2026-07-05T14:58:20-07:00",
          "tree_id": "fb969f8caaedfa6855adcbbe21b6ac5d61f91540",
          "url": "https://github.com/wallstop/fortress-rollback/commit/c52785fabc851f0f2d5e71381d1a1b561040f34b"
        },
        "date": 1783288980301,
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
            "range": "± 1",
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
            "value": 477,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 711,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1033,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 126140,
            "range": "± 3715",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 44875,
            "range": "± 260",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1245,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 107",
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
          "id": "fabdf182ca95636ab4c91b7cb74e45f8f8e467c2",
          "message": "Hardening M3 §6.6-pre.2: per-peer clock skew (H-SKEW precondition) (#202)\n\nAdds per-peer clock skew to the DST harness. Every prior fleet run shared one clock, so clock rate skew was unsimulatable (H-SKEW blind). Each session now gets a clock running at an integer-ratio rate relative to the network's real-time base — deterministic across platforms (u128 integer math, no float). Test-infra only.\n\n- test_clock.rs: TestClock::as_skewed_protocol_clock(num, den) — skewed = base_start + base_elapsed*num/den (saturating). Unit tests pin the scaling (2x/0.5x/1x/frozen) and reject den=0.\n- schedule.rs: SimConfig.clock_skew_ppm: Vec<i32> (#[serde(default)] empty = no skew; existing seeds bit-identical, no schema bump; serde-default test).\n- harness/mod.rs: per-peer clock injection (exact base at 0 ppm; frozen at -1_000_000; ppm < -1_000_000 rejected up front).\n- fleet.rs: clock_skew_is_tolerated_and_alters_execution (n in {2,4}) over a constant 40ms-delay mesh. Peer 0 at +10%: mesh stays byte-consistent (game logic is per-(step,peer), not clock-driven) while the skew alters timing-gated execution (red-verified premise); determinism holds.\n\nH-SKEW precondition infra + short-run tolerance observation; the 0.1%-over-an-hour drift experiment is a long-run probe, still owed. Also unblocks the symmetric H-OSC case (per-peer clock differences source the transient advantage symmetric delay alone doesn't).\n\nReviewed by an adversarial sub-agent (verdict: solid, determinism airtight, no underflow, no must-fix) + GitHub Copilot (2 rounds: frozen-clock semantics, scaling unit test) + Cursor Bugbot (SUCCESS).",
          "timestamp": "2026-07-05T15:50:35-07:00",
          "tree_id": "3001bfc89284622de87b05e7353689d1ee09fc14",
          "url": "https://github.com/wallstop/fortress-rollback/commit/fabdf182ca95636ab4c91b7cb74e45f8f8e467c2"
        },
        "date": 1783292113267,
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 456,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 706,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1031,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 122377,
            "range": "± 3854",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45262,
            "range": "± 266",
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
          "id": "493c54b1f61dae7d6ea2d22217fe4ef7c4999e54",
          "message": "Hardening M3: clock-skew consistency probe + honest H-SKEW status (#203)\n\nA long-run clock-skew consistency probe, and an honest status update on hypothesis H-SKEW.\n\nWhat this validly shows: per-peer clock skew (peer 0 at +0.1% over 10k steps) does not break mesh state consistency or liveness — the skew shifts timing-gated behavior (RTT gauge, quality-report/keepalive cadence) and the mesh still confirms a byte-identical prefix, peers in step. Extends the short +10% clock_skew_is_tolerated_and_alters_execution observation to a realistic magnitude over a long run.\n\nH-SKEW is NOT executed here — it remains OWED. An earlier revision claimed to *falsify* H-SKEW; the adversarial review correctly rejected that. This harness advances every peer one frame per step (lockstep) and reads the clock only for timestamps, so H-SKEW's rate-drift mechanism (fast clock -> faster frame production -> ~43 frames/hour accumulation) is structurally absent: the frame-advantage delta floor(half_rtt*fps/1000) stays 1 from 0% through ~+11% skew, so average_frame_advantage never reaches MIN_RECOMMENDATION at any ppm. 'Zero recommendations' was a tautology, not a falsification. Testing H-SKEW needs a skew-gated frame model (advance each peer at a rate driven by its own skewed clock) — recorded in PLAN.md §13/§6.6-pre.2.\n\n- fleet.rs: clock_skew_long_run_schedule + clock_skew_holds_consistency_over_a_long_run (#[ignore]d). Asserts consistency+liveness and that the confirmed-frame spread stays bounded (the first sign skew ever leaked into frame production).\n\nReviewed by an adversarial sub-agent (which correctly rejected the original falsification claim) + GitHub Copilot (3 rounds) + Cursor Bugbot (SUCCESS).",
          "timestamp": "2026-07-05T16:45:33-07:00",
          "tree_id": "5fdeedfaf8fdf997fecb7b53d2885cdc8a7a92d5",
          "url": "https://github.com/wallstop/fortress-rollback/commit/493c54b1f61dae7d6ea2d22217fe4ef7c4999e54"
        },
        "date": 1783295409216,
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
            "value": 110,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 174,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 467,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 719,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1045,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 122234,
            "range": "± 400",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45242,
            "range": "± 504",
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
            "range": "± 89",
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
          "id": "f736b737550cdf07c3c24fb292550c148af9b5ca",
          "message": "Hardening M3 §6.6-pre.6h: event-loss oracle — D9 fleet coverage via a starved app model (#204)\n\nFleet-level coverage of D9 event-discard telemetry: a starved-peer app model (never drains P2PSession::events) fills the bounded event_queue so the session trims and record_event_discard fires. Test proves fire (starved peer overflows) + neutralize (all-draining discards nothing); both pass the full oracle. Two #[serde(default)] SimConfig fields (starve_events, event_queue_size) keep existing seeds bit-identical. Adversarially reviewed; Copilot + Bugbot clean.",
          "timestamp": "2026-07-05T18:10:36-07:00",
          "tree_id": "2870a142bcb8858b77c20e5e710e5bb061e1d581",
          "url": "https://github.com/wallstop/fortress-rollback/commit/f736b737550cdf07c3c24fb292550c148af9b5ca"
        },
        "date": 1783300577526,
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
            "value": 96,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 140,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 569,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 922,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1334,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 97966,
            "range": "± 788",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 39996,
            "range": "± 344",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 867,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 869,
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
          "id": "ec3cff5b9d12d8cde692de5b31d063ba06f60488",
          "message": "Hardening M3 §6.2(c): bounded post-heal liveness + (i) metastability report (#205)\n\n## Summary\n\nLands the **§6.2 (c) bounded post-heal liveness** oracle invariant and\nthe **(i) metastability report** — the quantitative recovery check the\nDST harness was missing.\n\nUntil now the oracle's only liveness check was **(c-lite)**: at\nend-of-run every non-dead peer must be `Running` and confirmed ≥ 30.\nThat is a coarse *absolute* bar — a peer can stall for most of the run,\nrecover in the final stretch, and still clear it. **(c)** adds a\n*bounded* bar: after the last `HealAll`, every **live** peer's\n`confirmed_frame` must advance by at least **G = 10** frames within\n**B** steps of the heal. It catches a peer pinned post-heal (or a mutual\ndeadlock) that (c-lite) misses.\n\n**(i)** surfaces the metastability verdict explicitly:\n`recovered_within_b: Some(true)` = every live peer recovered,\n`Some(false)` = healed but at least one peer stayed pinned (the\nmetastable case), `None` = inert (no heal) or indeterminate (post-heal\nwindow < B).\n\n## The bound B (derived, not arbitrary)\n\n`RECOVERY_WINDOW_MS = 4000ms` folds the documented compound worst-case\nrecovery path — a peer that dropped during a partition must, after the\nheal, time its endpoints out, re-run sync, then re-fold gossip:\n\n```\ndisconnect_timeout           2000 ms   (builder.rs:43, protocol/mod.rs:1254)\n+ sync-retry burst (5 × 200)  1000 ms   (SyncConfig::default, protocol/mod.rs:6431-6435)\n+ gossip settle  (3 × 200)     600 ms   (keepalive cadence, protocol/mod.rs:1237)\n→ round up                    4000 ms   →  250 steps at 16ms\n```\n\nThis is the same ~250-step budget the schedule generator already\nreserves as its post-heal drain window. B is computed from `step_dt`\n(`recovery_window_steps()`), so it is the same wall-clock bound at any\nstep granularity. **G = 10** is a floor, not a rate target: `>\nmax_prediction` (8) so a mere prediction-window refill does not clear\nit, `< MIN_END_CONFIRMED` (30) so (c) stays complementary to (c-lite).\n\n## Both directions, with measured margins\n\nFour tests (`fleet.rs`), sharing one builder that differs only by stall\nduration — a clean red-green premise:\n\n- **passes when a hitched peer recovers** — peer 1 hitches 60 steps then\ncatches up; every peer advances **~191** frames post-heal →\n`Some(true)`, (c) passes. Also asserts (c) actually *ran* (anchors\nsampled per peer, each clears the floor) — not silently inert.\n- **inert without a heal** — no `HealAll` ⇒ `None`, empty anchors, no\n`PostHealLiveness`. Pins that `heal_at` alone (without a `HealAll`\nevent) is not a heal.\n- **fires on a pinned peer** — peer 1 frozen across the whole window\nadvances **0**, the three ContinueWithout survivors advance **90** each\n→ (c) fires on peer 1 *only*, `Some(false)`. The G=10 threshold sits\nunambiguously between 0 and 90.\n- **reports non-recovery under Halt** — a partition healed under `Halt`\nnever recovers (all peers halt) → `Some(false)`: the headline\nmetastability case, a true non-recovery.\n\n## Design: always-on, no schema bump\n\n(c) runs on **every** healing schedule (no opt-in), so the existing\nsmoke/tcp/hitch/split-brain fleets gain it for free — **verified: all\n2411 tests stay green**. The change adds no `SimConfig`/`Schedule`/serde\nfields (only harness-internal anchors + `RunReport`/`Verdict` fields),\nand the anchors reuse the already-trace-folded per-`(step,peer)`\n`confirmed` read — no new session call, no RNG draw — so existing seeds\nreplay **bit-identically**. Killed peers are excluded (dead, not\npinned); a stalled peer is live and *is* checked. `heals` is decided by\nan actual `HealAll` event, not `heal_at < steps`.\n\n## Changes (test-infra only — no `src/`, no public API, no changelog)\n\n- **`schedule.rs`**: `RECOVERY_WINDOW_MS` + `recovery_window_steps()`.\n- **`oracle.rs`**: `PostHealLiveness` failure, `HealLiveness` inputs\n(Default = inert), `set_heal_liveness`, the (c) check in `finalize`,\n`POST_HEAL_MIN_ADVANCE`, `Verdict::recovered_within_b`.\n- **`mod.rs`**: heal-anchored confirmed snapshots + 3 `RunReport`\nfields; `expect_pass` repro now prints `recovered_within_b`.\n- **`fleet.rs`**: the builder + four tests.\n\n## Validation\n\n- `cargo clippy --workspace --all-targets --features tokio,json` — clean\n- full test suite: **2411 passed**; the four new tests pass with the\nmargins above\n- `python3 scripts/ci/agent-preflight.py --auto-fix` — all checks pass\n\nReviewed by a Plan agent (design) + an adversarial sub-agent (always-on\nsoundness, negative-control teeth, B/G derivation, anchor-vector\nalignment, non-monotonicity) + GitHub Copilot.\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Test-only simulation harness and oracle changes; production library\ncode is untouched.\n> \n> **Overview**\n> Extends the **DST simulation harness** with **§6.2 (c) bounded\npost-heal liveness** and an **(i) metastability** signal, complementing\nexisting end-of-run **(c-lite)** checks.\n> \n> After the last real **`HealAll`** event (not `schedule.heal_at`\nalone), the runner samples each peer’s **`confirmed_frame`** at heal and\nat **B** steps later (**`RECOVERY_WINDOW_MS` / `step_dt`** via\n**`recovery_window_steps()`**). Live peers must advance by at least **G\n= 10** frames in that span or get **`PostHealLiveness`**; **`Verdict` /\n`RunReport`** expose **`recovered_within_b`** as **`Some(true/false)`**\nor **`None`** when the check is inert or indeterminate (no heal, window\nshorter than G, all peers dead).\n> \n> **`fleet.rs`** adds red/green tests (recovering hitch, pinned peer,\nno-heal inert, coarse `step_dt`, Halt non-recovery) plus **`step_dt_ms\n>= 1`** validation. No **`src/`** or public API changes.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nadf545113d910128792595ad9f7b688fde220293. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\n---------\n\nCo-authored-by: Claude Opus 4.8 (1M context) <noreply@anthropic.com>",
          "timestamp": "2026-07-06T08:51:36-07:00",
          "tree_id": "e507a75b64dfb5664b48b11d83af451e173a0324",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ec3cff5b9d12d8cde692de5b31d063ba06f60488"
        },
        "date": 1783353379668,
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
            "value": 95,
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
            "value": 546,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 905,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1317,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 98028,
            "range": "± 887",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 39822,
            "range": "± 435",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 867,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 868,
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
          "id": "fbffe891654fe7f1aacf76aa4299a4889b4b4ff1",
          "message": "[codex] Hardening M3 GracefulRemove lifecycle op (#206)\n\n## Summary\n\nAdds the M3 simulation lifecycle operation\n`ScheduleEvent::GracefulRemove { by, target }`, backed by the real\n`P2PSession::remove_player` API path. The deterministic simulation\nrunner now applies the removal on one live survivor, detaches/retires\nthe target, and lets the remaining mesh learn the drop through protocol\ngossip.\n\nThis also folds in adversarial harness hardening found during review:\n\n- checksum mismatch metrics now feed the oracle directly, so a starved\nevent queue cannot hide a checksum-only desync\n- materialized/corpus schedules are validated at `run()` time for player\nbounds, sorted/in-run events, post-heal faults, exact directed-link\ncoverage, self links, and duplicates\n- overlapping `PeerStall` events keep the later deadline instead of\nshortening an existing stall\n\n## Validation\n\n- `cargo fmt`\n- `cargo clippy --workspace --all-targets --features tokio,json`\n- `cargo nextest run --no-capture graceful_remove`\n- `cargo nextest run --no-capture\nchecksum_mismatch_metric_catches_starved_desync_event\nrun_rejects_invalid_materialized_schedule_invariants\noverlapping_peer_stalls_keep_the_later_deadline`\n- `cargo nextest run --no-capture simulation::` — 60 passed, 2388\nskipped\n- `cargo nextest run --no-capture` — 2391 passed, 57 skipped\n- `cargo nextest run --features hot-join --no-capture` — 2630 passed, 57\nskipped\n- `npx markdownlint 'PLAN.md'\n'progress/session-76-m3-graceful-remove-harness-hardening.md' --config\n.markdownlint.json --fix`\n- `python3 scripts/ci/agent-preflight.py --auto-fix`\n\nNo production `src/` changes and no public API changes; no changelog\nentry needed.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Changes are confined to simulation tests and harness/oracle code; no\nproduction src/ or public API changes, so rollout risk is low aside from\nstricter schedule validation for hand-authored corpora.\n> \n> **Overview**\n> Introduces **`ScheduleEvent::GracefulRemove`** (schedule schema v5):\nthe runner calls **`P2PSession::remove_player`** on one survivor,\ndetaches the target, and marks it retired like **`PeerKill`** so\nsurvivors stay byte-consistent under **`ContinueWithout`**. Fleet tests\ncover convergence, determinism, oracle negative controls, and\nmalformed-event validation.\n> \n> **Harness/oracle hardening:** **`run()`** now rejects invalid\nmaterialized schedules (player bounds, sorted in-run events, full\ndirected link mesh, no self/duplicate links, no faults after last\n**`HealAll`**). Overlapping **`PeerStall`** deadlines use **`max`**, not\noverwrite. The oracle fails on **`checksums_mismatched`** metrics when\nevent queues are starved, and **`SessionError`** records the failing\noperation (e.g. **`remove_player`**, **`set_input_delay`**).\nRetired-peer wording generalizes the alive mask beyond kills only.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n6521e5a39380db1e413e3fb7b036507aab7b0210. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-07-06T12:08:41-07:00",
          "tree_id": "1c172bd063fc1bbba002870206e53b77ea17ebca",
          "url": "https://github.com/wallstop/fortress-rollback/commit/fbffe891654fe7f1aacf76aa4299a4889b4b4ff1"
        },
        "date": 1783365177568,
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
            "value": 95,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 130,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 364,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 584,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 863,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 96165,
            "range": "± 574",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 37566,
            "range": "± 284",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1090,
            "range": "± 429",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1243,
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
      }
    ]
  }
}