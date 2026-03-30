window.BENCHMARK_DATA = {
  "lastUpdate": 1774828900992,
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
          "id": "c15a7b213e156f7945443aaf76cf629a137fc728",
          "message": "Fix wiki (#49)\n\n## Description\n\n<!-- Provide a clear and concise description of your changes. -->\n<!-- What problem does this solve? Why is this change needed? -->\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2025-12-29T10:12:03-08:00",
          "tree_id": "d10488420e0f2f708b3b7c17682a3a80fe2df270",
          "url": "https://github.com/wallstop/fortress-rollback/commit/c15a7b213e156f7945443aaf76cf629a137fc728"
        },
        "date": 1767032270533,
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
            "value": 100,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 151,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 485,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 695,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1014,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 103787,
            "range": "± 617",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27542,
            "range": "± 796",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
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
          "id": "26893b7aa5bbb130c372a279e00bb5ec99593a6c",
          "message": "chore(deps): bump z3 from 0.19.6 to 0.19.7 (#42)\n\nBumps [z3](https://github.com/prove-rs/z3.rs) from 0.19.6 to 0.19.7.\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/11b153cdd4cf75af8d4ea2e47368f053b74603e4\"><code>11b153c</code></a>\nchore: release (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/485\">#485</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/b6b3f92716bb18c2478f7e6868c2d5f157203e56\"><code>b6b3f92</code></a>\nfeat: allow configuring tls provider for <code>gh-release</code> and\n<code>bundled</code> (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/486\">#486</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/b5d606409e10655f661e9f64415c2af01466c79d\"><code>b5d6064</code></a>\nfeat: Add check_and_get_model method to Solver (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/484\">#484</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/prove-rs/z3.rs/compare/z3-v0.19.6...z3-v0.19.7\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=z3&package-manager=cargo&previous-version=0.19.6&new-version=0.19.7)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2025-12-29T14:40:19-08:00",
          "tree_id": "61190f14dab957a11b30759b62b3a2b0993112cc",
          "url": "https://github.com/wallstop/fortress-rollback/commit/26893b7aa5bbb130c372a279e00bb5ec99593a6c"
        },
        "date": 1767048287075,
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
            "value": 99,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 151,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 477,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 713,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1044,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 104175,
            "range": "± 967",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27872,
            "range": "± 923",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
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
          "id": "eaecf6aa411de117d5e1d3d6d3e6c156e374c9f5",
          "message": "chore(deps): bump serde_json from 1.0.147 to 1.0.148 (#44)\n\n[//]: # (dependabot-start)\n⚠️  **Dependabot is rebasing this PR** ⚠️ \n\nRebasing might not happen immediately, so don't worry if this takes some\ntime.\n\nNote: if you make any changes to this PR yourself, they will take\nprecedence over the rebase.\n\n---\n\n[//]: # (dependabot-end)\n\nBumps [serde_json](https://github.com/serde-rs/json) from 1.0.147 to\n1.0.148.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/serde-rs/json/releases\">serde_json's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v1.0.148</h2>\n<ul>\n<li>Update <code>zmij</code> dependency to 1.0</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/8b291c4c5620476d6834c69fbfb24d13a24d4596\"><code>8b291c4</code></a>\nRelease 1.0.148</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/1aefe152735f1b11ce7f641f8e86448d227163bf\"><code>1aefe15</code></a>\nUpdate to zmij 1.0</li>\n<li>See full diff in <a\nhref=\"https://github.com/serde-rs/json/compare/v1.0.147...v1.0.148\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=serde_json&package-manager=cargo&previous-version=1.0.147&new-version=1.0.148)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2025-12-29T14:41:29-08:00",
          "tree_id": "503a66a19f02767d8f331179a03596346ecc905e",
          "url": "https://github.com/wallstop/fortress-rollback/commit/eaecf6aa411de117d5e1d3d6d3e6c156e374c9f5"
        },
        "date": 1767048385775,
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 147,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 547,
            "range": "± 41",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 787,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1109,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102805,
            "range": "± 486",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27856,
            "range": "± 1616",
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
            "value": 1554,
            "range": "± 88",
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
          "id": "b43925ca0463aee2ee09585186b4d39d6875b2b2",
          "message": "chore(deps): bump tokio from 1.48.0 to 1.49.0 (#59)\n\nBumps [tokio](https://github.com/tokio-rs/tokio) from 1.48.0 to 1.49.0.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/tokio-rs/tokio/releases\">tokio's\nreleases</a>.</em></p>\n<blockquote>\n<h2>Tokio v1.49.0</h2>\n<h1>1.49.0 (January 3rd, 2026)</h1>\n<h3>Added</h3>\n<ul>\n<li>net: add support for <code>TCLASS</code> option on IPv6 (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7781\">#7781</a>)</li>\n<li>runtime: stabilize <code>runtime::id::Id</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7125\">#7125</a>)</li>\n<li>task: implement <code>Extend</code> for <code>JoinSet</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7195\">#7195</a>)</li>\n<li>task: stabilize the <code>LocalSet::id()</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7776\">#7776</a>)</li>\n</ul>\n<h3>Changed</h3>\n<ul>\n<li>net: deprecate <code>{TcpStream,TcpSocket}::set_linger</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7752\">#7752</a>)</li>\n</ul>\n<h3>Fixed</h3>\n<ul>\n<li>macros: fix the hygiene issue of <code>join!</code> and\n<code>try_join!</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7766\">#7766</a>)</li>\n<li>runtime: revert &quot;replace manual vtable definitions with\nWake&quot; (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7699\">#7699</a>)</li>\n<li>sync: return <code>TryRecvError::Disconnected</code> from\n<code>Receiver::try_recv</code> after <code>Receiver::close</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7686\">#7686</a>)</li>\n<li>task: remove unnecessary trait bounds on the <code>Debug</code>\nimplementation (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7720\">#7720</a>)</li>\n</ul>\n<h3>Unstable</h3>\n<ul>\n<li>fs: handle <code>EINTR</code> in <code>fs::write</code> for io-uring\n(<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7786\">#7786</a>)</li>\n<li>fs: support io-uring with <code>tokio::fs::read</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7696\">#7696</a>)</li>\n<li>runtime: disable io-uring on <code>EPERM</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7724\">#7724</a>)</li>\n<li>time: add alternative timer for better multicore scalability (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7467\">#7467</a>)</li>\n</ul>\n<h3>Documented</h3>\n<ul>\n<li>docs: fix a typos in <code>bounded.rs</code> and\n<code>park.rs</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7817\">#7817</a>)</li>\n<li>io: add <code>SyncIoBridge</code> cross-references to\n<code>copy</code> and <code>copy_buf</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7798\">#7798</a>)</li>\n<li>io: doc that <code>AsyncWrite</code> does not inherit from\n<code>std::io::Write</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7705\">#7705</a>)</li>\n<li>metrics: clarify that <code>num_alive_tasks</code> is not strongly\nconsistent (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7614\">#7614</a>)</li>\n<li>net: clarify the cancellation safety of the\n<code>TcpStream::peek</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7305\">#7305</a>)</li>\n<li>net: clarify the drop behavior of <code>unix::OwnedWriteHalf</code>\n(<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7742\">#7742</a>)</li>\n<li>net: clarify the platform-dependent backlog in\n<code>TcpSocket</code> docs (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7738\">#7738</a>)</li>\n<li>runtime: mention <code>LocalRuntime</code> in\n<code>new_current_thread</code> docs (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7820\">#7820</a>)</li>\n<li>sync: add missing period to <code>mpsc::Sender::try_send</code> docs\n(<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7721\">#7721</a>)</li>\n<li>sync: clarify the cancellation safety of\n<code>oneshot::Receiver</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7780\">#7780</a>)</li>\n<li>sync: improve the docs for the <code>errors</code> of mpsc (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7722\">#7722</a>)</li>\n<li>task: add example for <code>spawn_local</code> usage on local\nruntime (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7689\">#7689</a>)</li>\n</ul>\n<p><a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7125\">#7125</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/7125\">tokio-rs/tokio#7125</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7195\">#7195</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/7195\">tokio-rs/tokio#7195</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7305\">#7305</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/7305\">tokio-rs/tokio#7305</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7467\">#7467</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/7467\">tokio-rs/tokio#7467</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7614\">#7614</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/7614\">tokio-rs/tokio#7614</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7686\">#7686</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/7686\">tokio-rs/tokio#7686</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7689\">#7689</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/7689\">tokio-rs/tokio#7689</a></p>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/e3b89bbefa7564e2eba2fb9f849ef7bf87d60fad\"><code>e3b89bb</code></a>\nchore: prepare Tokio v1.49.0 (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7824\">#7824</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/4f577b84e939c8d427d79fdc73919842d8735de2\"><code>4f577b8</code></a>\nMerge 'tokio-1.47.3' into 'master'</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/f320197693ee09e28f1fca0e55418081adcdfc25\"><code>f320197</code></a>\nchore: prepare Tokio v1.47.3 (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7823\">#7823</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/ea6b144cd1042d6841a7830b18f2df77c3db904b\"><code>ea6b144</code></a>\nci: freeze rustc on nightly-2025-01-25 in <code>netlify.toml</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7652\">#7652</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/264e703296bccd6783a438815d91055d4517099b\"><code>264e703</code></a>\nMerge <code>tokio-1.43.4</code> into <code>tokio-1.47.x</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7822\">#7822</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/dfb0f00838ca1986dee04a54a6299d35b0a4072c\"><code>dfb0f00</code></a>\nchore: prepare Tokio v1.43.4 (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7821\">#7821</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/4a91f197b03dc335010fffcf0e0c14e1f4011b42\"><code>4a91f19</code></a>\nci: fix wasm32-wasip1 tests (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7788\">#7788</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/601c383ab6def5a6d2f95a434c95a97b65059628\"><code>601c383</code></a>\nci: upgrade FreeBSD from 14.2 to 14.3 (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7758\">#7758</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/484cb52d8d21cb8156decbeba9569651fcc09d0d\"><code>484cb52</code></a>\nsync: return <code>TryRecvError::Disconnected</code> from\n<code>Receiver::try_recv</code> after `Re...</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/16f20c34ed9bc11eb1e7cdec441ab844b198d2cd\"><code>16f20c3</code></a>\nrt: mention <code>LocalRuntime</code> in <code>new_current_thread</code>\ndocs (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7820\">#7820</a>)</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/tokio-rs/tokio/compare/tokio-1.48.0...tokio-1.49.0\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=tokio&package-manager=cargo&previous-version=1.48.0&new-version=1.49.0)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-01-05T21:02:36-08:00",
          "tree_id": "cd88d8276ec084a9f2c37c18da76034754a5e174",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b43925ca0463aee2ee09585186b4d39d6875b2b2"
        },
        "date": 1767676052630,
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
            "value": 87,
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
            "value": 628,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 910,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1356,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 96738,
            "range": "± 348",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24302,
            "range": "± 402",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 867,
            "range": "± 2",
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
          "id": "fd74f54215d01e0f11d77ceacaabfc5658c3df6d",
          "message": "chore(deps): bump clap from 4.5.53 to 4.5.54 (#57)\n\nBumps [clap](https://github.com/clap-rs/clap) from 4.5.53 to 4.5.54.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/releases\">clap's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v4.5.54</h2>\n<h2>[4.5.54] - 2026-01-02</h2>\n<h3>Fixes</h3>\n<ul>\n<li><em>(help)</em> Move <code>[default]</code> to its own paragraph\nwhen <code>PossibleValue::help</code> is present in\n<code>--help</code></li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/blob/master/CHANGELOG.md\">clap's\nchangelog</a>.</em></p>\n<blockquote>\n<h2>[4.5.54] - 2026-01-02</h2>\n<h3>Fixes</h3>\n<ul>\n<li><em>(help)</em> Move <code>[default]</code> to its own paragraph\nwhen <code>PossibleValue::help</code> is present in\n<code>--help</code></li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/194c676f60b916506f94f70decdbf319af5d1ec6\"><code>194c676</code></a>\nchore: Release</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/44838f6606fa015140c65a2d35971c1e9b269e26\"><code>44838f6</code></a>\ndocs: Update changelog</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/0f59d55ff6b132cd59cd252442ce47078494be07\"><code>0f59d55</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6027\">#6027</a>\nfrom Alpha1337k/master</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/e2aa2f07d1cd50412de51b51a7cc897e80e0b92f\"><code>e2aa2f0</code></a>\nFeat: Add catch-all on external subcommands for zsh</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/b9c0aee9f28c5ad72932225bd730260f9bbe1fc6\"><code>b9c0aee</code></a>\nFeat: Add external subcommands test to suite</li>\n<li>See full diff in <a\nhref=\"https://github.com/clap-rs/clap/compare/clap_complete-v4.5.53...clap_complete-v4.5.54\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=clap&package-manager=cargo&previous-version=4.5.53&new-version=4.5.54)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-01-05T21:02:27-08:00",
          "tree_id": "59fa41ad77487efac24d46e7f2e46dc0b1c01ba5",
          "url": "https://github.com/wallstop/fortress-rollback/commit/fd74f54215d01e0f11d77ceacaabfc5658c3df6d"
        },
        "date": 1767676056022,
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
            "value": 89,
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
            "value": 622,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 954,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1344,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 96664,
            "range": "± 859",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24124,
            "range": "± 263",
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
            "range": "± 10",
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
          "id": "5811045b9b1ba4aab8b0cced96b7ebdc8395edbf",
          "message": "chore(deps): bump serial_test from 3.2.0 to 3.3.1 (#58)\n\nBumps [serial_test](https://github.com/palfrey/serial_test) from 3.2.0\nto 3.3.1.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/palfrey/serial_test/releases\">serial_test's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v3.3.1</h2>\n<p>docs.rs removed a feature we use in <a\nhref=\"https://redirect.github.com/rust-lang/rust/pull/138907\">rust-lang/rust#138907</a>.\n<a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/148\">palfrey/serial_test#148</a>\n(which is the entire content of this release) adds a CI step to check we\ndon't break it in the future, and fixes the issue.</p>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/palfrey/serial_test/compare/v3.3.0...v3.3.1\">https://github.com/palfrey/serial_test/compare/v3.3.0...v3.3.1</a></p>\n<h2>v3.3.0</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Explicit testing for tokio multi-thread by <a\nhref=\"https://github.com/palfrey\"><code>@​palfrey</code></a> in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/122\">palfrey/serial_test#122</a></li>\n<li>Remove an unneeded explicit lifetime by <a\nhref=\"https://github.com/palfrey\"><code>@​palfrey</code></a> in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/124\">palfrey/serial_test#124</a></li>\n<li>docs: fixed the link to the shield by <a\nhref=\"https://github.com/operagxoksana\"><code>@​operagxoksana</code></a>\nin <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/125\">palfrey/serial_test#125</a></li>\n<li>Permit non-empty function returns by <a\nhref=\"https://github.com/palfrey\"><code>@​palfrey</code></a> in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/128\">palfrey/serial_test#128</a></li>\n<li>Add support for crate parameter by <a\nhref=\"https://github.com/palfrey\"><code>@​palfrey</code></a> in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/130\">palfrey/serial_test#130</a></li>\n<li>Add use serial_test::serial to Readme.md example by <a\nhref=\"https://github.com/APN-Pucky\"><code>@​APN-Pucky</code></a> in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/135\">palfrey/serial_test#135</a></li>\n<li>Fix elided lifetime warnings by <a\nhref=\"https://github.com/palfrey\"><code>@​palfrey</code></a> in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/138\">palfrey/serial_test#138</a></li>\n<li>Add docs about &quot;path&quot; for file_serial/parallel by <a\nhref=\"https://github.com/palfrey\"><code>@​palfrey</code></a> in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/137\">palfrey/serial_test#137</a></li>\n<li>Don't depend on the whole futures crate by <a\nhref=\"https://github.com/bilelmoussaoui\"><code>@​bilelmoussaoui</code></a>\nin <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/141\">palfrey/serial_test#141</a></li>\n<li>Add is_locked_file_serially by <a\nhref=\"https://github.com/palfrey\"><code>@​palfrey</code></a> in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/139\">palfrey/serial_test#139</a></li>\n<li>Add relative path and better file_serial testing by <a\nhref=\"https://github.com/palfrey\"><code>@​palfrey</code></a> in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/143\">palfrey/serial_test#143</a></li>\n<li>Add std feature to wasm-bindgen-test to avoid breaking dep updates\nby <a href=\"https://github.com/palfrey\"><code>@​palfrey</code></a> in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/145\">palfrey/serial_test#145</a></li>\n<li>Add some more logging around relative paths by <a\nhref=\"https://github.com/palfrey\"><code>@​palfrey</code></a> in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/147\">palfrey/serial_test#147</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a\nhref=\"https://github.com/operagxoksana\"><code>@​operagxoksana</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/125\">palfrey/serial_test#125</a></li>\n<li><a href=\"https://github.com/APN-Pucky\"><code>@​APN-Pucky</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/135\">palfrey/serial_test#135</a></li>\n<li><a\nhref=\"https://github.com/bilelmoussaoui\"><code>@​bilelmoussaoui</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/141\">palfrey/serial_test#141</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/palfrey/serial_test/compare/v3.2.0...v3.3.0\">https://github.com/palfrey/serial_test/compare/v3.2.0...v3.3.0</a></p>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/0771eb0e1e1e3fa9147b37536cd339073f0478fe\"><code>0771eb0</code></a>\n3.3.1</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/ace6ebf0eff78d7c3027bce72b9f418a9d352c28\"><code>ace6ebf</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/palfrey/serial_test/issues/148\">#148</a>\nfrom palfrey/docsrs-testing</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/349375d1e2bbbbe4f9eb6426b322f3de8fadd112\"><code>349375d</code></a>\nImprove feature flag docs</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/f9deb97f055ffec1d5aafb4f68c284138a584a63\"><code>f9deb97</code></a>\nRemove doc_auto_cfg</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/d5a4c09043b10dc2c74873686637f8af511475b5\"><code>d5a4c09</code></a>\nReset cargo.lock</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/89253b17d753f9a4b98f2c0b60e5b6f0590b910f\"><code>89253b1</code></a>\nTest docs-rs will work</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/5e84cd48e2078b5a109589f9283647a2f278417e\"><code>5e84cd4</code></a>\n3.3.0</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/abfc053ccb4dc6544c4b47b02ce2545c7183bfab\"><code>abfc053</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/palfrey/serial_test/issues/147\">#147</a>\nfrom palfrey/non-absolute-file-checking</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/38d6b05aa04c490156434fac2c17516cfe51e599\"><code>38d6b05</code></a>\nRefactor feature name for serial_test_test</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/2620c791d76b092fa606d358103426a99e4c5f1c\"><code>2620c79</code></a>\ntest-all-features needs more escaping</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/palfrey/serial_test/compare/v3.2.0...v3.3.1\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=serial_test&package-manager=cargo&previous-version=3.2.0&new-version=3.3.1)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-01-05T21:02:54-08:00",
          "tree_id": "3d1071dd7588f81c3510b40dcf05078e68c234c2",
          "url": "https://github.com/wallstop/fortress-rollback/commit/5811045b9b1ba4aab8b0cced96b7ebdc8395edbf"
        },
        "date": 1767676169075,
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
            "value": 91,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 139,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 618,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 952,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1344,
            "range": "± 49",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 97425,
            "range": "± 1622",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24504,
            "range": "± 912",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 867,
            "range": "± 17",
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
          "id": "6b476153010b52a24606508861d7c34ed38b6c7f",
          "message": "Minor improvements (#56)\n\n## Description\n\n&check; Remove a debug assert\n&check; More structured error messages\n&check; More determinism in Chaos Socket\n&check; Higher quality proofs\n&check; More CI/CD coverage\n&check; Flaky test fixes\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->\n\n---------\n\nCo-authored-by: Copilot <175728472+Copilot@users.noreply.github.com>",
          "timestamp": "2026-01-22T16:08:44-08:00",
          "tree_id": "75529495a4d74546413eccdd9fbd15c74235daa5",
          "url": "https://github.com/wallstop/fortress-rollback/commit/6b476153010b52a24606508861d7c34ed38b6c7f"
        },
        "date": 1769127285372,
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
            "value": 99,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 149,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 504,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 747,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1065,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 103812,
            "range": "± 929",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27568,
            "range": "± 935",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 53",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
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
          "id": "fd615115f040eda22dc606b63900efcd07f18a81",
          "message": "chore(deps): bump serde_json from 1.0.148 to 1.0.149 (#60)\n\nBumps [serde_json](https://github.com/serde-rs/json) from 1.0.148 to\n1.0.149.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/serde-rs/json/releases\">serde_json's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v1.0.149</h2>\n<ul>\n<li>Align arbitrary_precision number strings with zmij's formatting (<a\nhref=\"https://redirect.github.com/serde-rs/json/issues/1306\">#1306</a>,\nthanks <a href=\"https://github.com/b41sh\"><code>@​b41sh</code></a>)</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/4f6dbfac79647d032b0997b5ab73022340c6dab7\"><code>4f6dbfa</code></a>\nRelease 1.0.149</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/f3df680098007496f5580903890892d51116d129\"><code>f3df680</code></a>\nTouch up PR 1306</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/e16730ff445bc38c04537109d99e80c594f8150c\"><code>e16730f</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/serde-rs/json/issues/1306\">#1306</a>\nfrom b41sh/fix-float-number-display</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/eeb2bcd3f2fd2300de21381e23b3cebd33bfca30\"><code>eeb2bcd</code></a>\nAlign <code>arbitrary_precision</code> number strings with zmij’s\nformatting</li>\n<li>See full diff in <a\nhref=\"https://github.com/serde-rs/json/compare/v1.0.148...v1.0.149\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=serde_json&package-manager=cargo&previous-version=1.0.148&new-version=1.0.149)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-01-22T16:28:17-08:00",
          "tree_id": "1792ac5bda787f9f44fcf2a1b901cfd6bea472f2",
          "url": "https://github.com/wallstop/fortress-rollback/commit/fd615115f040eda22dc606b63900efcd07f18a81"
        },
        "date": 1769128385703,
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
            "value": 99,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 145,
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
            "value": 723,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1066,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102019,
            "range": "± 713",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27209,
            "range": "± 877",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1243,
            "range": "± 18",
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
          "id": "b91eb1a2e86ed20c76fa56262af323b5590386b1",
          "message": "chore(deps): bump js-sys from 0.3.83 to 0.3.85 in /loom-tests (#63)\n\nBumps [js-sys](https://github.com/wasm-bindgen/wasm-bindgen) from 0.3.83\nto 0.3.85.\n<details>\n<summary>Commits</summary>\n<ul>\n<li>See full diff in <a\nhref=\"https://github.com/wasm-bindgen/wasm-bindgen/commits\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=js-sys&package-manager=cargo&previous-version=0.3.83&new-version=0.3.85)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-01-22T20:21:25-08:00",
          "tree_id": "80ccf5620daedfaa0a122d8e77a925acb8dd4f4f",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b91eb1a2e86ed20c76fa56262af323b5590386b1"
        },
        "date": 1769142402991,
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
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 145,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 494,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 712,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1080,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 103497,
            "range": "± 1291",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27601,
            "range": "± 1067",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1592,
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
          "id": "f4529bf7679bb02bcc6ce7dbba9aa8b3bed8757a",
          "message": "chore(deps): bump tracing from 0.1.43 to 0.1.44 in /loom-tests (#64)\n\nBumps [tracing](https://github.com/tokio-rs/tracing) from 0.1.43 to\n0.1.44.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/tokio-rs/tracing/releases\">tracing's\nreleases</a>.</em></p>\n<blockquote>\n<h2>tracing 0.1.44</h2>\n<h3>Fixed</h3>\n<ul>\n<li>Fix <code>record_all</code> panic (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>)</li>\n</ul>\n<h3>Changed</h3>\n<ul>\n<li><code>tracing-core</code>: updated to 0.1.36 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>)</li>\n</ul>\n<p><a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/pull/3432\">tokio-rs/tracing#3432</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/pull/3440\">tokio-rs/tracing#3440</a></p>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/2d55f6faf9be83e7e4634129fb96813241aac2b8\"><code>2d55f6f</code></a>\nchore: prepare tracing 0.1.44 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3439\">#3439</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/10a9e838a35e6ded79d66af246be2ee05417136d\"><code>10a9e83</code></a>\nchore: prepare tracing-core 0.1.36 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/ee82cf92a8c750f98cfb7a417cc8defb37e26a00\"><code>ee82cf9</code></a>\ntracing: fix record_all panic (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/9978c3663bcd58de14b3cf089ad24cb63d00a922\"><code>9978c36</code></a>\nchore: prepare tracing-mock 0.1.0-beta.3 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3429\">#3429</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/cc44064b3a41cb586bd633f8a024354928e25819\"><code>cc44064</code></a>\nchore: prepare tracing-subscriber 0.3.22 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3428\">#3428</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/tokio-rs/tracing/compare/tracing-0.1.43...tracing-0.1.44\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=tracing&package-manager=cargo&previous-version=0.1.43&new-version=0.1.44)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-01-22T20:39:15-08:00",
          "tree_id": "db8ca30f238d6d39f73a986311f1142f582817bf",
          "url": "https://github.com/wallstop/fortress-rollback/commit/f4529bf7679bb02bcc6ce7dbba9aa8b3bed8757a"
        },
        "date": 1769143426745,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 145,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 500,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 741,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1094,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 101992,
            "range": "± 519",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27407,
            "range": "± 873",
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
            "value": 1554,
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
          "id": "304e1ed0b481ceb1f1cdf42093a550edf3b6ea86",
          "message": "Update GitHub pages render issue, update fortress v ggrs (#65)\n\n## Description\n\n&check; Update fortress v ggrs comparison to be more accurate\n&check; Fix some rendering issues related to code blocks in github pages\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2026-01-23T14:33:04-08:00",
          "tree_id": "af8168733d9d0a2432aeb7e980df096a9efa3490",
          "url": "https://github.com/wallstop/fortress-rollback/commit/304e1ed0b481ceb1f1cdf42093a550edf3b6ea86"
        },
        "date": 1769207873242,
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
            "value": 87,
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
            "value": 641,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 957,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1373,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 94990,
            "range": "± 1100",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24430,
            "range": "± 247",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 867,
            "range": "± 2",
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
          "id": "4db745da9691cf2eb66b0107068ad7c88556516d",
          "message": "chore(deps): bump clap from 4.5.54 to 4.5.55 (#68)\n\nBumps [clap](https://github.com/clap-rs/clap) from 4.5.54 to 4.5.55.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/releases\">clap's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v4.5.55</h2>\n<h2>[4.5.55] - 2026-01-27</h2>\n<h3>Fixes</h3>\n<ul>\n<li>Fix inconsistency in precedence between positionals with a\n<code>value_terminator(&quot;--&quot;)</code> and escapes\n(<code>--</code>) where <code>./foo -- bar</code> means the first arg is\nempty, rather than escaping future args</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/blob/master/CHANGELOG.md\">clap's\nchangelog</a>.</em></p>\n<blockquote>\n<h2>[4.5.55] - 2026-01-27</h2>\n<h3>Fixes</h3>\n<ul>\n<li>Fix inconsistency in precedence between positionals with a\n<code>value_terminator(&quot;--&quot;)</code> and escapes\n(<code>--</code>) where <code>./foo -- bar</code> means the first arg is\nempty, rather than escaping future args</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/4c039309b614ee4523c67a243afc38af11860de9\"><code>4c03930</code></a>\nchore: Release</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/fb948a25ffde7f108acc682d2751976e80ab100b\"><code>fb948a2</code></a>\ndocs: Update changelog</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/0f602396a338455e2782963b8c8fb20240a6a87b\"><code>0f60239</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6057\">#6057</a>\nfrom GilShoshan94/master</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/83d4206ff16b489895e8a171a3bdb2e39d7d3e1f\"><code>83d4206</code></a>\ntest: Update fixture to cover all cases + styling</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/b13274d869ffa53910a6d16728546b1ca9161b2d\"><code>b13274d</code></a>\nfix: Rename <code>pvs</code> to <code>dvs</code> for default values</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/df92ea02bb4aa79b2738d7a8a86dcbfab417b7dd\"><code>df92ea0</code></a>\nfeat(help): Allow styling for inline context</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/0e535e518d56df3f0b808d2f1977aba290e193c3\"><code>0e535e5</code></a>\nchore(deps): Update compatible (dev) (<a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6054\">#6054</a>)</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/de57287f303db0e0b29f1bcca05fef50ef011225\"><code>de57287</code></a>\nchore(deps): Update Rust Stable to v1.88 (<a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6048\">#6048</a>)</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/5504a134684fc1148e08d8e6919ca16e13ed83a4\"><code>5504a13</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6047\">#6047</a>\nfrom clap-rs/revert-6045-cleanup-docsrs</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/c1c243c924570052857d85206c7ee282e273a961\"><code>c1c243c</code></a>\nRevert &quot;Cleanup docs.rs related issues&quot;</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/clap-rs/clap/compare/clap_complete-v4.5.54...clap_complete-v4.5.55\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=clap&package-manager=cargo&previous-version=4.5.54&new-version=4.5.55)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-01-27T20:14:01-08:00",
          "tree_id": "f6dc3733b12082cd1b7d2275c19e58b8ebd54e9c",
          "url": "https://github.com/wallstop/fortress-rollback/commit/4db745da9691cf2eb66b0107068ad7c88556516d"
        },
        "date": 1769573951155,
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
            "value": 89,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 132,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 643,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 972,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1387,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 94758,
            "range": "± 264",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24294,
            "range": "± 185",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 681,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 872,
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
          "id": "4796ff6618072d5f073f87dc874a1877351f2f52",
          "message": "Various feedback based on first impl (#69)\n\n## Description\n\nAddressing various pain points based on initial integration\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [x] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [x ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [x] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->\n\n---------\n\nCo-authored-by: Copilot <175728472+Copilot@users.noreply.github.com>",
          "timestamp": "2026-01-28T19:35:32-08:00",
          "tree_id": "d5b21932a313b6fab98ac779c8efd9ed948d8971",
          "url": "https://github.com/wallstop/fortress-rollback/commit/4796ff6618072d5f073f87dc874a1877351f2f52"
        },
        "date": 1769658003199,
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 145,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 504,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 748,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1077,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 104466,
            "range": "± 518",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27518,
            "range": "± 828",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
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
          "id": "0098310b7b63e3e5fd999d8ca5c6139132338fa1",
          "message": "0.3.0 release (#70)\n\n## Description\n\nBump version\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2026-01-28T20:09:17-08:00",
          "tree_id": "a6472ee8e202a266ffea5ab52ce0800ecaef70d5",
          "url": "https://github.com/wallstop/fortress-rollback/commit/0098310b7b63e3e5fd999d8ca5c6139132338fa1"
        },
        "date": 1769660042282,
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
            "value": 100,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 145,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 490,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 731,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1062,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 103842,
            "range": "± 884",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27307,
            "range": "± 913",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
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
          "id": "a80ed6134f42114ced171a7e44f8814af3f21a58",
          "message": "More ergonomics (#71)\n\n## Description\n\n&check; More ergonomic helpers to assist in real game usage\n&check; Address banner not working in github pages\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] ✨ New feature (non-breaking change that adds functionality)\n- [x] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [x] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2026-01-30T12:47:31-08:00",
          "tree_id": "5da849bd266a0bcebc645e679e4e8d4653becb43",
          "url": "https://github.com/wallstop/fortress-rollback/commit/a80ed6134f42114ced171a7e44f8814af3f21a58"
        },
        "date": 1769806322744,
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
            "value": 99,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 145,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 496,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 747,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1038,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102993,
            "range": "± 721",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27336,
            "range": "± 1049",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
            "range": "± 82",
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
          "id": "b82b309155056bff46732f20e91a44a892de5e5c",
          "message": "chore(deps): bump clap from 4.5.55 to 4.5.56 (#72)\n\nBumps [clap](https://github.com/clap-rs/clap) from 4.5.55 to 4.5.56.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/releases\">clap's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v4.5.56</h2>\n<h2>[4.5.56] - 2026-01-29</h2>\n<h3>Fixes</h3>\n<ul>\n<li>On conflict error, don't show conflicting arguments in the\nusage</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/blob/master/CHANGELOG.md\">clap's\nchangelog</a>.</em></p>\n<blockquote>\n<h2>[4.5.56] - 2026-01-29</h2>\n<h3>Fixes</h3>\n<ul>\n<li>On conflict error, don't show conflicting arguments in the\nusage</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/9cec1007acdc3cd990feded4322a4bccd2fd471c\"><code>9cec100</code></a>\nchore: Release</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/00e72e06f46e2c21e5bb4dd82aa5fca02a9e5c16\"><code>00e72e0</code></a>\ndocs: Update changelog</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/c7848ff6fc3f8e0f7b66eaee10d44b43eea54538\"><code>c7848ff</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6094\">#6094</a>\nfrom epage/home</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/60184fb76a3d88277f89430402d01a121feb858c\"><code>60184fb</code></a>\nfeat(complete): Expand ~ in native completions</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/09969d3c1af9dd22fb944c09f8b1c27274cad824\"><code>09969d3</code></a>\nchore(deps): Update Rust Stable to v1.89 (<a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6093\">#6093</a>)</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/520beb5ec2d2bb5dd11912d27127df4e97027965\"><code>520beb5</code></a>\nchore: Release</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/2bd8ab3c009fc975db28209c3c3fb526364342ae\"><code>2bd8ab3</code></a>\ndocs: Update changelog</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/220875b58511028ba9cd38f7195b8b3315b72d0d\"><code>220875b</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6091\">#6091</a>\nfrom epage/possible</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/e5eb6c9d84efac5d660322e92dbbc0158266602d\"><code>e5eb6c9</code></a>\nfix(help): Integrate 'Possible Values:' into 'Arg::help'</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/594a771030e43df8c806ea1a029862339739a0f3\"><code>594a771</code></a>\nrefactor(help): Make empty tracking more consistent</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/clap-rs/clap/compare/clap_complete-v4.5.55...clap_complete-v4.5.56\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=clap&package-manager=cargo&previous-version=4.5.55&new-version=4.5.56)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-01-30T12:54:24-08:00",
          "tree_id": "56f062c3b412cfa1062e4c53ed11280924bdfdb1",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b82b309155056bff46732f20e91a44a892de5e5c"
        },
        "date": 1769806826569,
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
            "value": 1,
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
            "value": 162,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 552,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 813,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1174,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 110701,
            "range": "± 1227",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 29080,
            "range": "± 895",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1324,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1667,
            "range": "± 110",
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
          "id": "129b490f3b0a60a6e3a9f7c2c65eb963e1390399",
          "message": "v4 release (#73)\n\n## Description\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2026-01-30T13:16:26-08:00",
          "tree_id": "1a792c6cfdc45f3c890db07d60349df347b8b154",
          "url": "https://github.com/wallstop/fortress-rollback/commit/129b490f3b0a60a6e3a9f7c2c65eb963e1390399"
        },
        "date": 1769808058327,
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
            "value": 102,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 155,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 517,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 754,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1105,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 97204,
            "range": "± 1580",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 36753,
            "range": "± 611",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1406,
            "range": "± 2",
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
          "id": "028eadff890aca63453cb3454b4871670c70df3c",
          "message": "Documentation Accuracy + Fmt impls (#74)\n\n## Description\n\n- Hopefully clarify documentation\n- Add various additional impls like `std::fmt` to various public types\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [x] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2026-01-31T13:46:57-08:00",
          "tree_id": "0b13a5f8a7f91e4e926d532c305ef770be196f65",
          "url": "https://github.com/wallstop/fortress-rollback/commit/028eadff890aca63453cb3454b4871670c70df3c"
        },
        "date": 1769896284054,
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
            "value": 99,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 149,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 516,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 744,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1108,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 103971,
            "range": "± 706",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27593,
            "range": "± 1214",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
            "range": "± 79",
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
          "id": "7abd66fe66e761dab5f1d4d40d59f3520c632987",
          "message": "Bump version (0.4.1) (#75)\n\n## Description\n\n-> 0.4.1\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [x ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [x] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2026-01-31T14:13:15-08:00",
          "tree_id": "75e86e0f79e72caba15993485de137ed8f2cae1e",
          "url": "https://github.com/wallstop/fortress-rollback/commit/7abd66fe66e761dab5f1d4d40d59f3520c632987"
        },
        "date": 1769897894587,
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
            "value": 100,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 149,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 504,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 759,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1083,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 104032,
            "range": "± 1163",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27623,
            "range": "± 899",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
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
          "id": "be03ad2b495cee8d3ef9450a9ebccf63c79d7068",
          "message": "chore(deps): bump clap from 4.5.56 to 4.5.57 (#78)\n\nBumps [clap](https://github.com/clap-rs/clap) from 4.5.56 to 4.5.57.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/releases\">clap's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v4.5.57</h2>\n<h2>[4.5.57] - 2026-02-03</h2>\n<h3>Fixes</h3>\n<ul>\n<li>Regression from 4.5.55 where having an argument with\n<code>.value_terminator(&quot;--&quot;)</code> caused problems with an\nargument with <code>.last(true)</code></li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/blob/master/CHANGELOG.md\">clap's\nchangelog</a>.</em></p>\n<blockquote>\n<h2>[4.5.57] - 2026-02-03</h2>\n<h3>Fixes</h3>\n<ul>\n<li>Regression from 4.5.55 where having an argument with\n<code>.value_terminator(&quot;--&quot;)</code> caused problems with an\nargument with <code>.last(true)</code></li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/69c0ddbbfb56db1bccbb5954b62bb89a567a3c8d\"><code>69c0ddb</code></a>\nchore: Release</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/8206bba73fd6c5d567cb95949fd1c3c6c48e4e20\"><code>8206bba</code></a>\ndocs: Update changelog</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/c109d67ea493823727411f60f354edb3d83117ee\"><code>c109d67</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6104\">#6104</a>\nfrom epage/hide</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/9d7f2128f77023941b53b7cfc311120a2ead75a2\"><code>9d7f212</code></a>\nfix(complete): Hide dot files on dynamic completer</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/77b3fdbbea64ae0b0b3a51309bcbb861360de8d1\"><code>77b3fdb</code></a>\ntest(complete): Show dot file behavior</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/f89b9b8d1b818a2eb3863745be48725ace2d8f12\"><code>f89b9b8</code></a>\ntest(derive): Make stable across upgrade</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/58eb8a937ac6ca4a59614dc26deedb6cfe16c424\"><code>58eb8a9</code></a>\nchore: Release</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/10a2a7559b0663143d56c850c0c40ed31620cb5b\"><code>10a2a75</code></a>\ndocs: Update changelog</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/a42eebf56bf20d587347abb03105f95c98bfda51\"><code>a42eebf</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6103\">#6103</a>\nfrom epage/mut_subcommands</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/5335f54d73eef9276c13313661fcfffb720c87cf\"><code>5335f54</code></a>\nfeat: Add Command::mut_subcommands</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/clap-rs/clap/compare/clap_complete-v4.5.56...clap_complete-v4.5.57\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=clap&package-manager=cargo&previous-version=4.5.56&new-version=4.5.57)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-02-03T23:45:28-08:00",
          "tree_id": "74963318c7fbcee933aa721c37e14108a0dcfa03",
          "url": "https://github.com/wallstop/fortress-rollback/commit/be03ad2b495cee8d3ef9450a9ebccf63c79d7068"
        },
        "date": 1770191426859,
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
            "value": 88,
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
            "value": 615,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 923,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1391,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 103700,
            "range": "± 1754",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24450,
            "range": "± 201",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 681,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 871,
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
          "id": "14f9ad04cf2b437bb3df3d2fa4357ab4ea84e6b9",
          "message": "chore(deps): bump proptest from 1.9.0 to 1.10.0 (#80)\n\nBumps [proptest](https://github.com/proptest-rs/proptest) from 1.9.0 to\n1.10.0.\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/4814e510f61b402c94d7063086ed61fda732736f\"><code>4814e51</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/627\">#627</a>\nfrom proptest-rs/release-prep-1.10</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/8221c9a314ccb85209fc1a314736d0e97d1f8650\"><code>8221c9a</code></a>\nprep 1.10 (and other) release(s)</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/28961bf85a80183e3e8e032f8627ffe3e32493e1\"><code>28961bf</code></a>\nfix(macro): set <code>Config::test_name</code> to actual fn name (<a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/619\">#619</a>)</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/8fb08ffca8e70e16956f5d34fed35ab47f001e88\"><code>8fb08ff</code></a>\nUpdate trybuild requirement from =1.0.113 to =1.0.115 (<a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/624\">#624</a>)</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/5b7a435151b5755169e04561118c07b40f03e7c4\"><code>5b7a435</code></a>\nUpdate convert_case requirement from 0.6 to 0.11 (<a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/623\">#623</a>)</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/14d95fcd7f7800ee792a177628c4fcdc4bb03713\"><code>14d95fc</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/586\">#586</a>\nfrom regexident/range-subset-strategy</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/229e72393f07966965c88a2c24c281be1a6854d0\"><code>229e723</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/622\">#622</a>\nfrom ssanderson/proptest-macro-fixes</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/3663c38fc8572c90fa76f4d9de2c93dc955d88b7\"><code>3663c38</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/621\">#621</a>\nfrom wgyt/wgyt-patch</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/1fe04c9345768f37750b3eb557cae6ae7562c936\"><code>1fe04c9</code></a>\nFix import of <code>HashMap</code></li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/9c644db2becec9c77d038d7d877e444641f9aed7\"><code>9c644db</code></a>\nSupport returning TestCaseResult from #[property_test] tests.</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/proptest-rs/proptest/compare/v1.9.0...v1.10.0\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=proptest&package-manager=cargo&previous-version=1.9.0&new-version=1.10.0)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-02-04T20:23:40-08:00",
          "tree_id": "3be6127466149a6b45aac5004eba95b7a7be0ae1",
          "url": "https://github.com/wallstop/fortress-rollback/commit/14f9ad04cf2b437bb3df3d2fa4357ab4ea84e6b9"
        },
        "date": 1770265738309,
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
            "value": 87,
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
            "value": 632,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 943,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1355,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 96164,
            "range": "± 1859",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24831,
            "range": "± 1299",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 675,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 872,
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
          "id": "4e4a11d6cface7021090969cbd9d15a1974f713f",
          "message": "chore(deps): bump criterion from 0.8.1 to 0.8.2 (#79)\n\nBumps [criterion](https://github.com/criterion-rs/criterion.rs) from\n0.8.1 to 0.8.2.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/criterion-rs/criterion.rs/releases\">criterion's\nreleases</a>.</em></p>\n<blockquote>\n<h2>criterion-plot-v0.8.2</h2>\n<h3>Other</h3>\n<ul>\n<li>Update Readme</li>\n</ul>\n<h2>criterion-v0.8.2</h2>\n<h3>Fixed</h3>\n<ul>\n<li>don't build alloca on unsupported targets</li>\n</ul>\n<h3>Other</h3>\n<ul>\n<li><em>(deps)</em> bump crate-ci/typos from 1.40.0 to 1.43.0</li>\n<li>Fix panic with uniform iteration durations in benchmarks</li>\n<li>Update Readme</li>\n<li>Exclude development scripts from published package</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/criterion-rs/criterion.rs/blob/master/CHANGELOG.md\">criterion's\nchangelog</a>.</em></p>\n<blockquote>\n<h2><a\nhref=\"https://github.com/criterion-rs/criterion.rs/compare/criterion-v0.8.1...criterion-v0.8.2\">0.8.2</a>\n- 2026-02-04</h2>\n<h3>Fixed</h3>\n<ul>\n<li>don't build alloca on unsupported targets</li>\n</ul>\n<h3>Other</h3>\n<ul>\n<li><em>(deps)</em> bump crate-ci/typos from 1.40.0 to 1.43.0</li>\n<li>Fix panic with uniform iteration durations in benchmarks</li>\n<li>Update Readme</li>\n<li>Exclude development scripts from published package</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/criterion-rs/criterion.rs/commit/7f0d745532e3c7b2e11bbf9de9b911f91790d3b1\"><code>7f0d745</code></a>\nchore: release v0.8.2</li>\n<li><a\nhref=\"https://github.com/criterion-rs/criterion.rs/commit/4a467ce964052ae9bd9266c0706b470b817613e0\"><code>4a467ce</code></a>\nchore(deps): bump crate-ci/typos from 1.40.0 to 1.43.0</li>\n<li><a\nhref=\"https://github.com/criterion-rs/criterion.rs/commit/b277a751453cf9ce0595e41bddf819210a6d6e47\"><code>b277a75</code></a>\nFix panic with uniform iteration durations in benchmarks</li>\n<li><a\nhref=\"https://github.com/criterion-rs/criterion.rs/commit/828af1450d648c599a92a077b75e292747761d99\"><code>828af14</code></a>\nfix: don't build alloca on unsupported targets</li>\n<li><a\nhref=\"https://github.com/criterion-rs/criterion.rs/commit/b01316b76e42028f3b1cf3731f643bea7f354f39\"><code>b01316b</code></a>\nUpdate Readme</li>\n<li><a\nhref=\"https://github.com/criterion-rs/criterion.rs/commit/4c02a3b4e560fe1f296c0ed1e9b53e3154a3cac6\"><code>4c02a3b</code></a>\nExclude development scripts from published package</li>\n<li>See full diff in <a\nhref=\"https://github.com/criterion-rs/criterion.rs/compare/criterion-v0.8.1...criterion-v0.8.2\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=criterion&package-manager=cargo&previous-version=0.8.1&new-version=0.8.2)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-02-04T20:23:48-08:00",
          "tree_id": "fc4ac105479c5f9730ec6261cac803dcb5bb96a4",
          "url": "https://github.com/wallstop/fortress-rollback/commit/4e4a11d6cface7021090969cbd9d15a1974f713f"
        },
        "date": 1770265777718,
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
            "value": 99,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 149,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 502,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 720,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1092,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 103952,
            "range": "± 439",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27627,
            "range": "± 1025",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
            "range": "± 70",
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
          "id": "0abc14285ff1e3725f7eaf2da700ed852c0cc5f5",
          "message": "Syntax Sugar and Optimizations (#81)\n\n## Description\n\n&check; QoL syntax sugar improvements around player handlers\n&check; Optimizations - no longer allocate when querying player handles\non various paths\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] ✨ New feature (non-breaking change that adds functionality)\n- [x] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [x] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [x] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2026-02-05T14:33:30-08:00",
          "tree_id": "368aa8da3b5f04b31e668524a9af14d99b62cc56",
          "url": "https://github.com/wallstop/fortress-rollback/commit/0abc14285ff1e3725f7eaf2da700ed852c0cc5f5"
        },
        "date": 1770331119946,
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
            "value": 100,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 148,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 511,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 713,
            "range": "± 43",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1023,
            "range": "± 71",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102186,
            "range": "± 485",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27638,
            "range": "± 1609",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
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
          "id": "780e549e652cfa6eff408d2c9e31ae05522e0f88",
          "message": "v5 release (#82)\n\n## Description\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [x] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [x] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2026-02-05T16:45:01-08:00",
          "tree_id": "e6c950519523e2e9a9b16bdce26da494d262c546",
          "url": "https://github.com/wallstop/fortress-rollback/commit/780e549e652cfa6eff408d2c9e31ae05522e0f88"
        },
        "date": 1770338969772,
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
            "value": 100,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 148,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 492,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 715,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1057,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102328,
            "range": "± 590",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27459,
            "range": "± 820",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
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
          "id": "9e7d17e7ac499cf7d0c1d37bad87f26160fe454f",
          "message": "Session abstraction (#85)\n\n## Description\n\nAdd Session abstraction and optimizations\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] ✨ New feature (non-breaking change that adds functionality)\n- [x] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [x] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [x] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2026-02-07T07:34:37-08:00",
          "tree_id": "e43d6bfa4ff9ed7cc172711df0508a86ef7ae6bb",
          "url": "https://github.com/wallstop/fortress-rollback/commit/9e7d17e7ac499cf7d0c1d37bad87f26160fe454f"
        },
        "date": 1770478787123,
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
            "value": 157,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 437,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 687,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 991,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102320,
            "range": "± 464",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27626,
            "range": "± 838",
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
            "value": 1554,
            "range": "± 82",
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
          "id": "40990d27e5dcf7510a611ee187da6317cb9dc72f",
          "message": "Version updates (#86)\n\n## Description\n0.6.0 release\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->\n\n---------\n\nCo-authored-by: Copilot <175728472+Copilot@users.noreply.github.com>",
          "timestamp": "2026-02-07T08:12:49-08:00",
          "tree_id": "03a9d3f9d7d1023bbe2f1ea45764485368d56440",
          "url": "https://github.com/wallstop/fortress-rollback/commit/40990d27e5dcf7510a611ee187da6317cb9dc72f"
        },
        "date": 1770481036436,
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
            "value": 156,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 437,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 672,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 993,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102437,
            "range": "± 476",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27554,
            "range": "± 887",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
            "range": "± 67",
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
          "id": "a94e5f2d431be07e50182f79ff835120315d4922",
          "message": "Fix test bugs (#87)\n\n## Description\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2026-02-07T11:03:07-08:00",
          "tree_id": "0917efe481163df41a8f8887733599b32ac46bb4",
          "url": "https://github.com/wallstop/fortress-rollback/commit/a94e5f2d431be07e50182f79ff835120315d4922"
        },
        "date": 1770491256909,
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
            "value": 101,
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
            "value": 566,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 905,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1337,
            "range": "± 46",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 95383,
            "range": "± 280",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24186,
            "range": "± 250",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 679,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 868,
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
          "id": "176437cf10a40980eb914031b675b37433646e34",
          "message": "chore(deps): bump clap from 4.5.57 to 4.5.58 (#90)\n\nBumps [clap](https://github.com/clap-rs/clap) from 4.5.57 to 4.5.58.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/releases\">clap's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v4.5.58</h2>\n<h2>[4.5.58] - 2026-02-11</h2>\n</blockquote>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/blob/master/CHANGELOG.md\">clap's\nchangelog</a>.</em></p>\n<blockquote>\n<h2>[4.5.58] - 2026-02-11</h2>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/88f13cb4b0eed760139de41ecf80aefd19a707c1\"><code>88f13cb</code></a>\nchore: Release</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/fe2d731605e98597f241d4dd56950eb4226dfde9\"><code>fe2d731</code></a>\ndocs: Update changelog</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/b2567390457ce0b7ceab722a6318ba278f637a45\"><code>b256739</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6131\">#6131</a>\nfrom mernen/do-not-suggest-opts-after-escape</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/8aaf704f5679e2329a2f8048ff3cfad40696fde7\"><code>8aaf704</code></a>\nfix(complete): Do not suggest options after &quot;--&quot;</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/4a86fee1b523aeade43b628294a18a68df5ee165\"><code>4a86fee</code></a>\ntest(complete): Illustrate current behavior</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/281f8aec7ce468d677ae24bf5bc17d41e9c7cbcb\"><code>281f8ae</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6126\">#6126</a>\nfrom epage/p</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/3cbce42cc2115975432647c4238fa5dc9a2d662a\"><code>3cbce42</code></a>\ndocs(cookbook): Make typed-derive easier to maintain</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/9fd4dc9e4e6a6b2f5b696e8753b767a46e2aca7e\"><code>9fd4dc9</code></a>\ndocs(cookbook): Provide a custom TypedValueParser</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/8f8e8613459e3ccdd25051c97f011cd8d5e49ed9\"><code>8f8e861</code></a>\ndocs(cookbook): Add local enum to typed-derive</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/926bafef0b8860c4b437db0c41567fc270586089\"><code>926bafe</code></a>\ndocs(cookbook): Hint at overriding value_name</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/clap-rs/clap/compare/clap_complete-v4.5.57...clap_complete-v4.5.58\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=clap&package-manager=cargo&previous-version=4.5.57&new-version=4.5.58)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-02-12T14:09:23-08:00",
          "tree_id": "1481acd5baf7d048698560a906d50dd56ded46f3",
          "url": "https://github.com/wallstop/fortress-rollback/commit/176437cf10a40980eb914031b675b37433646e34"
        },
        "date": 1770934479383,
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
            "range": "± 5",
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
            "value": 1042,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102685,
            "range": "± 564",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27272,
            "range": "± 1110",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1244,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 93",
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
          "id": "05bcecb8927eb3581c04faaa3d79f2302da2cafb",
          "message": "chore(deps): bump z3 from 0.19.7 to 0.19.8 (#92)\n\nBumps [z3](https://github.com/prove-rs/z3.rs) from 0.19.7 to 0.19.8.\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/281c22f14cc12b314040799e12d64ed889cdc672\"><code>281c22f</code></a>\nchore: release (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/492\">#492</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/d5d112ddbcc1d55435a4dc9eb9af53d217495116\"><code>d5d112d</code></a>\nfix: Z3_SYS_BUNDLED_DIR_OVERRIDE had extra z3 (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/491\">#491</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/prove-rs/z3.rs/compare/z3-v0.19.7...z3-v0.19.8\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=z3&package-manager=cargo&previous-version=0.19.7&new-version=0.19.8)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-02-15T20:21:39-08:00",
          "tree_id": "6ba2b5212c6c206954bc403f137888ce71620373",
          "url": "https://github.com/wallstop/fortress-rollback/commit/05bcecb8927eb3581c04faaa3d79f2302da2cafb"
        },
        "date": 1771215978336,
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
            "value": 159,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 441,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 684,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1015,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102354,
            "range": "± 447",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27338,
            "range": "± 1115",
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
            "range": "± 100",
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
          "id": "df78c65f3a2cd8b14578803f12e0ef3c60c85d61",
          "message": "chore(deps): bump clap from 4.5.58 to 4.5.59 (#94)\n\nBumps [clap](https://github.com/clap-rs/clap) from 4.5.58 to 4.5.59.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/releases\">clap's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v4.5.59</h2>\n<h2>[4.5.59] - 2026-02-16</h2>\n<h3>Fixes</h3>\n<ul>\n<li><code>Command::ignore_errors</code> no longer masks help/version on\nsubcommands</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/blob/master/CHANGELOG.md\">clap's\nchangelog</a>.</em></p>\n<blockquote>\n<h2>[4.5.59] - 2026-02-16</h2>\n<h3>Fixes</h3>\n<ul>\n<li><code>Command::ignore_errors</code> no longer masks help/version on\nsubcommands</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/0bb3ad7e12e729be9f152391558689ac4fdd31ec\"><code>0bb3ad7</code></a>\nchore: Release</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/5cb5ce3873a882ba2a7d619864202eadef21fffa\"><code>5cb5ce3</code></a>\ndocs: Update changelog</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/245c8ba75a481250a48170f1add11532a7b7fd33\"><code>245c8ba</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6149\">#6149</a>\nfrom epage/wrap</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/dd17a418a9e6665c98ff6e0ba2a039fd1921988e\"><code>dd17a41</code></a>\nfix(help): Correctly calculate wrap points with ANSI escape codes</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/2cc4e350b9ea8955a9cf229405407426921e7871\"><code>2cc4e35</code></a>\ntest(ui): Avoid override term width</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/93e3559b1e4c8c81377f3598f7249b7708f4c379\"><code>93e3559</code></a>\nrefactor(help): Clarify that we're carrying over indentation</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/b1c46e633c04f5cb0d819b15f25c1fde1a6e42c4\"><code>b1c46e6</code></a>\nrefactor(help): Clarify var name</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/03b9b38df059c1a9a529f295e038f81de295627a\"><code>03b9b38</code></a>\ntest(help): Show styled wrapping behavior</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/c9a39a534c3e95926be272765bec48a80e5ea9e7\"><code>c9a39a5</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6146\">#6146</a>\nfrom clap-rs/renovate/actions-checkout-5.x</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/58599fb7bf865f8ec0a7a021dea8111f5dffe6d2\"><code>58599fb</code></a>\nchore(deps): Update actions/checkout action to v5</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/clap-rs/clap/compare/clap_complete-v4.5.58...clap_complete-v4.5.59\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=clap&package-manager=cargo&previous-version=4.5.58&new-version=4.5.59)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-02-16T20:27:21-08:00",
          "tree_id": "2167b2ba865a8677a9d901d5714393301b4a5849",
          "url": "https://github.com/wallstop/fortress-rollback/commit/df78c65f3a2cd8b14578803f12e0ef3c60c85d61"
        },
        "date": 1771302733623,
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
            "value": 159,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 432,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 689,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1000,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 104806,
            "range": "± 670",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27421,
            "range": "± 966",
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
            "range": "± 77",
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
          "id": "dc3930219a73b8398846878dc3c2970126d55159",
          "message": "chore(deps): bump clap from 4.5.59 to 4.5.60 (#95)\n\nBumps [clap](https://github.com/clap-rs/clap) from 4.5.59 to 4.5.60.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/releases\">clap's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v4.5.60</h2>\n<h2>[4.5.60] - 2026-02-19</h2>\n<h3>Fixes</h3>\n<ul>\n<li><em>(help)</em> Quote empty default values, possible values</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/blob/master/CHANGELOG.md\">clap's\nchangelog</a>.</em></p>\n<blockquote>\n<h2>[4.5.60] - 2026-02-19</h2>\n<h3>Fixes</h3>\n<ul>\n<li><em>(help)</em> Quote empty default values, possible values</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/33d24d844b11c0e926ae132e1af338ff070bdf4a\"><code>33d24d8</code></a>\nchore: Release</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/9332409f4a6c1d5c22064e839ec8e9bc040f3be7\"><code>9332409</code></a>\ndocs: Update changelog</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/b7adce5a17089596eecb2af6985e6503f2ffcd38\"><code>b7adce5</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6166\">#6166</a>\nfrom fabalchemy/fix-dynamic-powershell-completion</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/009bba44ec3d182028ec3a72f5b6f3e507827768\"><code>009bba4</code></a>\nfix(clap_complete): Improve powershell registration</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/d89d57dfb4bdd18930a40c6d7f4fadb23ee9c5b3\"><code>d89d57d</code></a>\nchore: Release</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/f18b67ec3d4ce6ac1acf115adaab2f16ab2ed3c7\"><code>f18b67e</code></a>\ndocs: Update changelog</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/9d218eb418526143c9110f734f78a608b8cf6440\"><code>9d218eb</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6165\">#6165</a>\nfrom epage/shirt</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/126440ca846613671e1dac98198b2ceb17dab2b0\"><code>126440c</code></a>\nfix(help): Correctly calculate padding for short-only args</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/9e3c05ef3800a3e638b8224a7881a81517a4f4db\"><code>9e3c05e</code></a>\ntest(help): Show panic with short, valueless arg</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/c9898d0fece98d8520d3dd954cf457b685b3308f\"><code>c9898d0</code></a>\ntest(help): Verify short with value</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/clap-rs/clap/compare/clap_complete-v4.5.59...clap_complete-v4.5.60\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=clap&package-manager=cargo&previous-version=4.5.59&new-version=4.5.60)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-02-19T20:25:41-08:00",
          "tree_id": "5a0e503cd68f08c38015508f3caac99a554b8163",
          "url": "https://github.com/wallstop/fortress-rollback/commit/dc3930219a73b8398846878dc3c2970126d55159"
        },
        "date": 1771561844172,
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
            "value": 160,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 426,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 688,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 999,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 122026,
            "range": "± 6439",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 42750,
            "range": "± 4434",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1586,
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
          "id": "2267fbbe2bbfac542689618a0a935de57b48e2bc",
          "message": "chore(deps): bump js-sys from 0.3.85 to 0.3.88 in /loom-tests (#96)\n\nBumps [js-sys](https://github.com/wasm-bindgen/wasm-bindgen) from 0.3.85\nto 0.3.88.\n<details>\n<summary>Commits</summary>\n<ul>\n<li>See full diff in <a\nhref=\"https://github.com/wasm-bindgen/wasm-bindgen/commits\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=js-sys&package-manager=cargo&previous-version=0.3.85&new-version=0.3.88)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-02-22T20:24:31-08:00",
          "tree_id": "5475096e7d7ff815c80b912e0d751632a267f35e",
          "url": "https://github.com/wallstop/fortress-rollback/commit/2267fbbe2bbfac542689618a0a935de57b48e2bc"
        },
        "date": 1771820967752,
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
            "value": 129,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 175,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 433,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 694,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1007,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102242,
            "range": "± 6927",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27569,
            "range": "± 856",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
            "range": "± 58",
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
          "id": "1e848c380db8ad1a2ce0d4b4a6d5ea78a20002e0",
          "message": "chore(deps): bump serial_test from 3.3.1 to 3.4.0 (#97)\n\nBumps [serial_test](https://github.com/palfrey/serial_test) from 3.3.1\nto 3.4.0.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/palfrey/serial_test/releases\">serial_test's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v3.4.0</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Implement inner attributes capability by <a\nhref=\"https://github.com/Carter12s\"><code>@​Carter12s</code></a> in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/150\">palfrey/serial_test#150</a></li>\n<li>Specify rust-version for workspace by <a\nhref=\"https://github.com/xtqqczze\"><code>@​xtqqczze</code></a> in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/152\">palfrey/serial_test#152</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/Carter12s\"><code>@​Carter12s</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/150\">palfrey/serial_test#150</a></li>\n<li><a href=\"https://github.com/xtqqczze\"><code>@​xtqqczze</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/palfrey/serial_test/pull/152\">palfrey/serial_test#152</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/palfrey/serial_test/compare/v3.3.1...v3.3.2\">https://github.com/palfrey/serial_test/compare/v3.3.1...v3.3.2</a></p>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/9a307f24c2e1eaa1dc0113a575cee48883849e3f\"><code>9a307f2</code></a>\n3.4.0</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/f5e47fd8f89c5c21ccdfe8d09095ca66806e4401\"><code>f5e47fd</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/palfrey/serial_test/issues/153\">#153</a>\nfrom palfrey/non-yanked-packages</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/93fc70c54d7e8d3d54431d4160d7abb5e4935c05\"><code>93fc70c</code></a>\nUpdate scc and futures-util to non-yanked</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/bda53c7e76b3db3d735e6c27de1aa2ea9b5b007f\"><code>bda53c7</code></a>\nRun cargo audit</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/27f36aac386096a176ebd6d1e07beca98a3a6bec\"><code>27f36aa</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/palfrey/serial_test/issues/152\">#152</a>\nfrom xtqqczze/rust-version</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/8633f7dc72c453a210d158f61eb6c6222cd3e36d\"><code>8633f7d</code></a>\nspecify rust-version for workspace</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/656280f425d06a66cdfd6a67f1997c66f693d904\"><code>656280f</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/palfrey/serial_test/issues/151\">#151</a>\nfrom palfrey/flag-doctests</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/71d9590ded004b19acefa8487415faa15070807e\"><code>71d9590</code></a>\nFlag #[test] in docs as non-running to sate clippy</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/e01cf3488c075c69dc6336da7a3bd2d984cae1f4\"><code>e01cf34</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/palfrey/serial_test/issues/150\">#150</a>\nfrom Carter12s/feature/implement-inner-atters</li>\n<li><a\nhref=\"https://github.com/palfrey/serial_test/commit/0fdbe254227f504c6a025435ad266a6d9d6747a5\"><code>0fdbe25</code></a>\nUpdate test exectations to match updated error message grammer</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/palfrey/serial_test/compare/v3.3.1...v3.4.0\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=serial_test&package-manager=cargo&previous-version=3.3.1&new-version=3.4.0)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-02-22T20:24:40-08:00",
          "tree_id": "5cb0efc5ccbb23418b3070208dcd002a2223db95",
          "url": "https://github.com/wallstop/fortress-rollback/commit/1e848c380db8ad1a2ce0d4b4a6d5ea78a20002e0"
        },
        "date": 1771820995272,
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
            "value": 158,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 435,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 684,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1008,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102810,
            "range": "± 3920",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27590,
            "range": "± 870",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
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
          "id": "79ac7f2e3522988d59a9266d1685dba15885862b",
          "message": "chore(deps): bump z3 from 0.19.8 to 0.19.9 (#98)\n\nBumps [z3](https://github.com/prove-rs/z3.rs) from 0.19.8 to 0.19.9.\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/f5640a3e2b0889740a666aca79757629a0b548ab\"><code>f5640a3</code></a>\nchore: release (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/497\">#497</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/d9542b0b12114c5ced1df987feea494cbc210464\"><code>d9542b0</code></a>\nchore: expand build flag documentation (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/499\">#499</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/ce50a037a210d2f12f8fdc63cb38e2445dfc7fef\"><code>ce50a03</code></a>\nfix: add rerun-if-env-changed for Z3_SYS_BUNDLED_DIR_OVERRIDE (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/496\">#496</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/prove-rs/z3.rs/compare/z3-v0.19.8...z3-v0.19.9\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=z3&package-manager=cargo&previous-version=0.19.8&new-version=0.19.9)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-02-22T20:24:53-08:00",
          "tree_id": "f4bad7736a900f4942c57a04d77eb40251dc403b",
          "url": "https://github.com/wallstop/fortress-rollback/commit/79ac7f2e3522988d59a9266d1685dba15885862b"
        },
        "date": 1771821042251,
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 168,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 457,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 727,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1055,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 97772,
            "range": "± 13234",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 36492,
            "range": "± 598",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1417,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1601,
            "range": "± 7",
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
          "id": "24e89ba3426222b3881545a97f82358bc729d238",
          "message": "chore(deps): bump z3 from 0.19.9 to 0.19.10 (#100)\n\nBumps [z3](https://github.com/prove-rs/z3.rs) from 0.19.9 to 0.19.10.\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/97f57a99ac7057f38c6c8cdeac74f178ae2c2e7e\"><code>97f57a9</code></a>\nchore: release (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/503\">#503</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/5631a0fd155e8babedf1cee4245a4d17cc5df7ae\"><code>5631a0f</code></a>\nchore: bump Z3 default version to 4.16.0 (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/504\">#504</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/3d11d7449dad53a6327233909a3ca52ce9110f98\"><code>3d11d74</code></a>\nfeat: algebraic numbers, polynomials, enhanced floats, AST vectors, and\nquant...</li>\n<li>See full diff in <a\nhref=\"https://github.com/prove-rs/z3.rs/compare/z3-v0.19.9...z3-v0.19.10\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=z3&package-manager=cargo&previous-version=0.19.9&new-version=0.19.10)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-02-24T20:23:59-08:00",
          "tree_id": "54027f33a9df55249d13b5a5f7621a83330efb48",
          "url": "https://github.com/wallstop/fortress-rollback/commit/24e89ba3426222b3881545a97f82358bc729d238"
        },
        "date": 1771993733433,
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
            "value": 158,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 456,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 706,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1041,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 105359,
            "range": "± 1165",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27598,
            "range": "± 1675",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
            "range": "± 65",
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
          "id": "cf93c7cf2f753786247e8d2313a2802744aa6e31",
          "message": "chore(deps): bump js-sys from 0.3.88 to 0.3.90 in /loom-tests (#99)\n\nBumps [js-sys](https://github.com/wasm-bindgen/wasm-bindgen) from 0.3.88\nto 0.3.90.\n<details>\n<summary>Commits</summary>\n<ul>\n<li>See full diff in <a\nhref=\"https://github.com/wasm-bindgen/wasm-bindgen/commits\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=js-sys&package-manager=cargo&previous-version=0.3.88&new-version=0.3.90)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-02-24T20:31:10-08:00",
          "tree_id": "1408de8d9b94247eb8c28f25bcfabb0a6dedc4b0",
          "url": "https://github.com/wallstop/fortress-rollback/commit/cf93c7cf2f753786247e8d2313a2802744aa6e31"
        },
        "date": 1771994144561,
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
            "value": 156,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 416,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 680,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 993,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 100253,
            "range": "± 2530",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 33515,
            "range": "± 7393",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1239,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
            "range": "± 82",
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
          "id": "55d1aefa9694d1a7e7d4480ac9c949d7f173ac94",
          "message": "chore(ci): bump actions/upload-artifact from 6 to 7 (#101)\n\nBumps\n[actions/upload-artifact](https://github.com/actions/upload-artifact)\nfrom 6 to 7.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/actions/upload-artifact/releases\">actions/upload-artifact's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v7.0.0</h2>\n<h2>v7 What's new</h2>\n<h3>Direct Uploads</h3>\n<p>Adds support for uploading single files directly (unzipped). Callers\ncan set the new <code>archive</code> parameter to <code>false</code> to\nskip zipping the file during upload. Right now, we only support single\nfiles. The action will fail if the glob passed resolves to multiple\nfiles. The <code>name</code> parameter is also ignored with this\nsetting. Instead, the name of the artifact will be the name of the\nuploaded file.</p>\n<h3>ESM</h3>\n<p>To support new versions of the <code>@actions/*</code> packages,\nwe've upgraded the package to ESM.</p>\n<h2>What's Changed</h2>\n<ul>\n<li>Add proxy integration test by <a\nhref=\"https://github.com/Link\"><code>@​Link</code></a>- in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/754\">actions/upload-artifact#754</a></li>\n<li>Upgrade the module to ESM and bump dependencies by <a\nhref=\"https://github.com/danwkennedy\"><code>@​danwkennedy</code></a> in\n<a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/762\">actions/upload-artifact#762</a></li>\n<li>Support direct file uploads by <a\nhref=\"https://github.com/danwkennedy\"><code>@​danwkennedy</code></a> in\n<a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/764\">actions/upload-artifact#764</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/Link\"><code>@​Link</code></a>- made\ntheir first contribution in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/754\">actions/upload-artifact#754</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/upload-artifact/compare/v6...v7.0.0\">https://github.com/actions/upload-artifact/compare/v6...v7.0.0</a></p>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/bbbca2ddaa5d8feaa63e36b76fdaad77386f024f\"><code>bbbca2d</code></a>\nSupport direct file uploads (<a\nhref=\"https://redirect.github.com/actions/upload-artifact/issues/764\">#764</a>)</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/589182c5a4cec8920b8c1bce3e2fab1c97a02296\"><code>589182c</code></a>\nUpgrade the module to ESM and bump dependencies (<a\nhref=\"https://redirect.github.com/actions/upload-artifact/issues/762\">#762</a>)</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/47309c993abb98030a35d55ef7ff34b7fa1074b5\"><code>47309c9</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/actions/upload-artifact/issues/754\">#754</a>\nfrom actions/Link-/add-proxy-integration-tests</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/02a8460834e70dab0ce194c64360c59dc1475ef0\"><code>02a8460</code></a>\nAdd proxy integration test</li>\n<li>See full diff in <a\nhref=\"https://github.com/actions/upload-artifact/compare/v6...v7\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=actions/upload-artifact&package-manager=github_actions&previous-version=6&new-version=7)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-02-26T20:10:21-08:00",
          "tree_id": "2ae2e8ee049309f596b4e60dd17cbd4ed5417ef6",
          "url": "https://github.com/wallstop/fortress-rollback/commit/55d1aefa9694d1a7e7d4480ac9c949d7f173ac94"
        },
        "date": 1772165696310,
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
            "value": 160,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 438,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 701,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 990,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 106529,
            "range": "± 894",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27382,
            "range": "± 1327",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1243,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 76",
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
          "id": "6eba6f3011a8f73caae8e28fe8a32a2ea19e565d",
          "message": "Potentially Fix Pipeline (Kani issues) (#109)\n\n## Description\n\n<!-- Provide a clear and concise description of your changes. -->\n<!-- What problem does this solve? Why is this change needed? -->\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2026-03-12T08:16:34-07:00",
          "tree_id": "ee6ad4851fdd09924f9ef2e9f6af7574cc5285b2",
          "url": "https://github.com/wallstop/fortress-rollback/commit/6eba6f3011a8f73caae8e28fe8a32a2ea19e565d"
        },
        "date": 1773328898680,
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
            "value": 142,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 178,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 429,
            "range": "± 10",
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
            "value": 984,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 103349,
            "range": "± 1237",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27874,
            "range": "± 981",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1243,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
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
          "id": "55f218b8ef28b8857f425c179c42d0e1d46c28ee",
          "message": "chore(deps): bump tokio from 1.49.0 to 1.50.0 (#105)\n\nBumps [tokio](https://github.com/tokio-rs/tokio) from 1.49.0 to 1.50.0.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/tokio-rs/tokio/releases\">tokio's\nreleases</a>.</em></p>\n<blockquote>\n<h2>Tokio v1.50.0</h2>\n<h1>1.50.0 (Mar 3rd, 2026)</h1>\n<h3>Added</h3>\n<ul>\n<li>net: add <code>TcpStream::set_zero_linger</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7837\">#7837</a>)</li>\n<li>rt: add <code>is_rt_shutdown_err</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7771\">#7771</a>)</li>\n</ul>\n<h3>Changed</h3>\n<ul>\n<li>io: add optimizer hint that <code>memchr</code> returns in-bounds\npointer (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7792\">#7792</a>)</li>\n<li>io: implement vectored writes for <code>write_buf</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7871\">#7871</a>)</li>\n<li>runtime: panic when <code>event_interval</code> is set to 0 (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7838\">#7838</a>)</li>\n<li>runtime: shorten default thread name to fit in Linux limit (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7880\">#7880</a>)</li>\n<li>signal: remember the result of <code>SetConsoleCtrlHandler</code>\n(<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7833\">#7833</a>)</li>\n<li>signal: specialize windows <code>Registry</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7885\">#7885</a>)</li>\n</ul>\n<h3>Fixed</h3>\n<ul>\n<li>io: always cleanup <code>AsyncFd</code> registration list on\nderegister (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7773\">#7773</a>)</li>\n<li>macros: remove (most) local <code>use</code> declarations in\n<code>tokio::select!</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7929\">#7929</a>)</li>\n<li>net: fix <code>GET_BUF_SIZE</code> constant for <code>target_os =\n&quot;android&quot;</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7889\">#7889</a>)</li>\n<li>runtime: avoid redundant unpark in current_thread scheduler (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7834\">#7834</a>)</li>\n<li>runtime: don't park in <code>current_thread</code> if\n<code>before_park</code> defers waker (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7835\">#7835</a>)</li>\n<li>io: fix write readiness on ESP32 on short writes (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7872\">#7872</a>)</li>\n<li>runtime: wake deferred tasks before entering\n<code>block_in_place</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7879\">#7879</a>)</li>\n<li>sync: drop rx waker when oneshot receiver is dropped (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7886\">#7886</a>)</li>\n<li>runtime: fix double increment of <code>num_idle_threads</code> on\nshutdown (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7910\">#7910</a>,\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7918\">#7918</a>,\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7922\">#7922</a>)</li>\n</ul>\n<h3>Unstable</h3>\n<ul>\n<li>fs: check for io-uring opcode support (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7815\">#7815</a>)</li>\n<li>runtime: avoid lock acquisition after uring init (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7850\">#7850</a>)</li>\n</ul>\n<h3>Documented</h3>\n<ul>\n<li>docs: update outdated unstable features section (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7839\">#7839</a>)</li>\n<li>io: clarify the behavior of <code>AsyncWriteExt::shutdown()</code>\n(<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7908\">#7908</a>)</li>\n<li>io: explain how to flush stdout/stderr (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7904\">#7904</a>)</li>\n<li>io: fix incorrect and confusing <code>AsyncWrite</code>\ndocumentation (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7875\">#7875</a>)</li>\n<li>rt: clarify the documentation of <code>Runtime::spawn</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7803\">#7803</a>)</li>\n<li>rt: fix missing quotation in docs (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7925\">#7925</a>)</li>\n<li>runtime: correct the default thread name in docs (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7896\">#7896</a>)</li>\n<li>runtime: fix <code>event_interval</code> doc (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7932\">#7932</a>)</li>\n<li>sync: clarify RwLock fairness documentation (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7919\">#7919</a>)</li>\n<li>sync: clarify that <code>recv</code> returns <code>None</code> once\nclosed and no more messages (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7920\">#7920</a>)</li>\n<li>task: clarify when to use <code>spawn_blocking</code> vs dedicated\nthreads (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7923\">#7923</a>)</li>\n<li>task: doc that task drops before <code>JoinHandle</code> completion\n(<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7825\">#7825</a>)</li>\n<li>signal: guarantee that listeners never return <code>None</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7869\">#7869</a>)</li>\n<li>task: fix task module feature flags in docs (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7891\">#7891</a>)</li>\n</ul>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/0273e45ead199dac7725faee1e3dc35a9c8753ab\"><code>0273e45</code></a>\nchore: prepare Tokio v1.50.0 (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7934\">#7934</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/e3ee4e58dc9bb7accf26dfd51b0a2146922b5269\"><code>e3ee4e5</code></a>\nchore: prepare tokio-macros v2.6.1 (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7943\">#7943</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/8c980ea75a0f8dd2799403777db700c2e8f4cda4\"><code>8c980ea</code></a>\nio: add <code>write_all_vectored</code> to <code>tokio-util</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7768\">#7768</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/e35fd6d6b7d9a8ba37ee621835ef91372c2565cb\"><code>e35fd6d</code></a>\nci: fix patch during clippy step (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7935\">#7935</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/03fe44c10302fdb55c29dbe5b08d4f8769c80272\"><code>03fe44c</code></a>\nruntime: fix <code>event_interval</code> doc (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7932\">#7932</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/d18e5dfbb0cdc28725bebb28cde80a6c11ee32bc\"><code>d18e5df</code></a>\nio: fix race in <code>Mock::poll_write</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7882\">#7882</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/f21f2693f02aec9a876ac2bd21566c85e15b682e\"><code>f21f269</code></a>\nruntime: fix race condition during the blocking pool shutdown (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7922\">#7922</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/d81e8f0acbdd7d866bce4f733b3545fd834c7840\"><code>d81e8f0</code></a>\nmacros: remove (most) local <code>use</code> declarations in\n<code>tokio::select!</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7929\">#7929</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/25e7f2641ef2555d688c267059431a2802805f1d\"><code>25e7f26</code></a>\nrt: fix missing quotation in docs (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7925\">#7925</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/e1a91ef114a301b542d810abab9956f2868861b9\"><code>e1a91ef</code></a>\nutil: fix typo in docs (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7926\">#7926</a>)</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/tokio-rs/tokio/compare/tokio-1.49.0...tokio-1.50.0\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=tokio&package-manager=cargo&previous-version=1.49.0&new-version=1.50.0)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-12T09:07:14-07:00",
          "tree_id": "0d24387267482134d74c3987f1d303a228a60d78",
          "url": "https://github.com/wallstop/fortress-rollback/commit/55f218b8ef28b8857f425c179c42d0e1d46c28ee"
        },
        "date": 1773331939799,
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
            "value": 425,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 679,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 988,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 104708,
            "range": "± 974",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27306,
            "range": "± 842",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1243,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
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
          "id": "bf97a402c8a9cf1f187c52f1c3fb872f2026e5f2",
          "message": "chore(deps): bump js-sys from 0.3.90 to 0.3.91 in /loom-tests (#103)\n\nBumps [js-sys](https://github.com/wasm-bindgen/wasm-bindgen) from 0.3.90\nto 0.3.91.\n<details>\n<summary>Commits</summary>\n<ul>\n<li>See full diff in <a\nhref=\"https://github.com/wasm-bindgen/wasm-bindgen/commits\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=js-sys&package-manager=cargo&previous-version=0.3.90&new-version=0.3.91)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-12T09:07:28-07:00",
          "tree_id": "dd67f476b87446d948c7ded5eec73a139f74ccc6",
          "url": "https://github.com/wallstop/fortress-rollback/commit/bf97a402c8a9cf1f187c52f1c3fb872f2026e5f2"
        },
        "date": 1773331989646,
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
            "value": 159,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 428,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 677,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 983,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102707,
            "range": "± 727",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27517,
            "range": "± 1063",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1243,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 93",
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
          "id": "ff0f8b975bc0f1f46c40fd3c051db002f9d3643f",
          "message": "chore(deps): bump clap from 4.5.60 to 4.6.0 (#111)\n\nBumps [clap](https://github.com/clap-rs/clap) from 4.5.60 to 4.6.0.\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/blob/master/CHANGELOG.md\">clap's\nchangelog</a>.</em></p>\n<blockquote>\n<h2>[4.6.0] - 2026-03-12</h2>\n<h3>Compatibility</h3>\n<ul>\n<li>Update MSRV to 1.85</li>\n</ul>\n<h2>[4.5.61] - 2026-03-12</h2>\n<h3>Internal</h3>\n<ul>\n<li>Update dependencies</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/9ab6dee710aa384e02ec5e9e2cfeadb2f35abf2a\"><code>9ab6dee</code></a>\nchore: Release</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/374a30dac685d492cbdae124e757afdb52dd47b6\"><code>374a30d</code></a>\ndocs: Update changelog</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/d0c8aabc000adc54fc39efa721e6caad035fc3da\"><code>d0c8aab</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6306\">#6306</a>\nfrom epage/update</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/686ce2f665f43f927c1dbd5ad63a2f989e503bb9\"><code>686ce2f</code></a>\nchore: Upgrade compatible</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/8203238de61f89b60ce1ca1672cfe20997d20a1e\"><code>8203238</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6305\">#6305</a>\nfrom epage/msrv</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/c774a892ba8bb703a9e77a16e6ebc6ff1c551868\"><code>c774a89</code></a>\ndocs: Reduce main's in doctests</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/73534f6ed3697b834743d283cedc7f529778d8a7\"><code>73534f6</code></a>\nchore: Upgrade to 2025 edition</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/dfe05a9bfe5bf49ec560e484c1abf50bcb55cd96\"><code>dfe05a9</code></a>\nchore: Bump MSRV to 1.85</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/8b41d0b8497ccaa0fb0d1d8a51f91ea2f62b3aa8\"><code>8b41d0b</code></a>\nchore: Release</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/518220f102cc34b2cf39c64efa35975a22341e36\"><code>518220f</code></a>\ndocs: Update changelog</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/clap-rs/clap/compare/clap_complete-v4.5.60...clap_complete-v4.6.0\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=clap&package-manager=cargo&previous-version=4.5.60&new-version=4.6.0)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-13T06:39:30-07:00",
          "tree_id": "02d53bb70efbb25b79caaddedcf260f1bb9b2336",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ff0f8b975bc0f1f46c40fd3c051db002f9d3643f"
        },
        "date": 1773409471358,
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
            "value": 103,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 141,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 546,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 886,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1322,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 105622,
            "range": "± 411",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24363,
            "range": "± 302",
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
            "value": 869,
            "range": "± 4",
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
          "id": "1144cca59d23c36fbf17a919cd95e63e41535aa5",
          "message": "chore(deps): bump z3 from 0.19.10 to 0.19.13 (#108)\n\nBumps [z3](https://github.com/prove-rs/z3.rs) from 0.19.10 to 0.19.13.\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/a189139b14228005cfe78cc4ca66f5bb95762cc7\"><code>a189139</code></a>\nchore: release (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/514\">#514</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/e2e07018fb38a229c370ed6dcbb31ae40771b728\"><code>e2e0701</code></a>\nfix(z3-sys): raise GitHub download timeout for gh-release (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/513\">#513</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/65c7a6fbbb017c5e875645641ea7a487b14a72b8\"><code>65c7a6f</code></a>\nchore(z3): release v0.19.12 (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/512\">#512</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/aa5557bfa09ef2886b1c861d1e719c8e99400e66\"><code>aa5557b</code></a>\nfeat: add <code>with</code> method to Tactic (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/511\">#511</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/b532c1305836c800374d682fe19562d9d3f3f4e9\"><code>b532c13</code></a>\nchore(z3): release v0.19.11 (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/507\">#507</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/026cd51597890b23ecaf80711470fbdd8090e491\"><code>026cd51</code></a>\nadded: FusedIterator and ExactSizeIterator for model/SortIter (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/509\">#509</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/582e1938e370492e16e489e8c6f40c3469fa3188\"><code>582e193</code></a>\nfix: standardize AstVector display/debug impl (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/508\">#508</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/b18d4a6834f765278065ebace8c6b6743b1ab2de\"><code>b18d4a6</code></a>\nfeat: add high-level API for model sorts/sort universes (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/506\">#506</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/prove-rs/z3.rs/compare/z3-v0.19.10...z3-v0.19.13\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=z3&package-manager=cargo&previous-version=0.19.10&new-version=0.19.13)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-13T08:22:01-07:00",
          "tree_id": "fe7aae6efca8fa40aa07602abae4ce73ca38d86c",
          "url": "https://github.com/wallstop/fortress-rollback/commit/1144cca59d23c36fbf17a919cd95e63e41535aa5"
        },
        "date": 1773415589490,
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
            "range": "± 2",
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
            "value": 439,
            "range": "± 10",
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
            "value": 1019,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102576,
            "range": "± 486",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27502,
            "range": "± 1171",
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
            "range": "± 112",
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
          "id": "0b8477996f99c018b4f0bbc10bdc0b18a71c60b3",
          "message": "Fix mutation testing (#113)\n\n## Description\n\n<!-- Provide a clear and concise description of your changes. -->\n<!-- What problem does this solve? Why is this change needed? -->\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --all-targets` with no\nwarnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2026-03-15T08:00:50-07:00",
          "tree_id": "0bd550e4a461bdc9b08648f73f508a06481a745c",
          "url": "https://github.com/wallstop/fortress-rollback/commit/0b8477996f99c018b4f0bbc10bdc0b18a71c60b3"
        },
        "date": 1773587123747,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 165,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 434,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 692,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1064,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102715,
            "range": "± 436",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27483,
            "range": "± 1108",
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
            "value": 1554,
            "range": "± 95",
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
          "id": "201cc6ccd615c17ffc5df3d7183028c2f8d716e3",
          "message": "chore(deps): bump tracing-subscriber from 0.3.22 to 0.3.23 (#115)\n\nBumps [tracing-subscriber](https://github.com/tokio-rs/tracing) from\n0.3.22 to 0.3.23.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/tokio-rs/tracing/releases\">tracing-subscriber's\nreleases</a>.</em></p>\n<blockquote>\n<h2>tracing-subscriber 0.3.23</h2>\n<h3>Fixed</h3>\n<ul>\n<li>Allow ansi sanitization to be disabled (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3484\">#3484</a>)</li>\n</ul>\n<p><a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3484\">#3484</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/pull/3484\">tokio-rs/tracing#3484</a></p>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/54ede4d5d85a536aed5485c5213011d9ec961935\"><code>54ede4d</code></a>\nchore: prepare tracing-subscriber 0.3.23 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3490\">#3490</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/37558d5f26340e999089bf3a680a800435332312\"><code>37558d5</code></a>\nsubscriber: allow ansi sanitization to be disabled (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3484\">#3484</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/efc690fa6bd1d9c3a57528b9bc8ac80504a7a6ed\"><code>efc690f</code></a>\ncore: add missing const (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3449\">#3449</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/0c32367cf9df27e750c4c81803de62a4e64e2ef1\"><code>0c32367</code></a>\ncore: Use const initializers instead of <code>once_cell</code></li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/9feb241133e55e70c7d4399689b8ef72f71d070f\"><code>9feb241</code></a>\ndocs: add arcswap reload crate to related (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3442\">#3442</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/2d55f6faf9be83e7e4634129fb96813241aac2b8\"><code>2d55f6f</code></a>\nchore: prepare tracing 0.1.44 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3439\">#3439</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/10a9e838a35e6ded79d66af246be2ee05417136d\"><code>10a9e83</code></a>\nchore: prepare tracing-core 0.1.36 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/ee82cf92a8c750f98cfb7a417cc8defb37e26a00\"><code>ee82cf9</code></a>\ntracing: fix record_all panic (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/9978c3663bcd58de14b3cf089ad24cb63d00a922\"><code>9978c36</code></a>\nchore: prepare tracing-mock 0.1.0-beta.3 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3429\">#3429</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/tokio-rs/tracing/compare/tracing-subscriber-0.3.22...tracing-subscriber-0.3.23\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=tracing-subscriber&package-manager=cargo&previous-version=0.3.22&new-version=0.3.23)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-20T05:23:36-07:00",
          "tree_id": "eed2d14e8d1482a7ed1e2d9d39e72861a4b06277",
          "url": "https://github.com/wallstop/fortress-rollback/commit/201cc6ccd615c17ffc5df3d7183028c2f8d716e3"
        },
        "date": 1774009717742,
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
            "value": 196,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 435,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 699,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1034,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102827,
            "range": "± 423",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27402,
            "range": "± 1053",
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
            "range": "± 69",
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
          "id": "dd673ceae4b6e245af0e426adfd251f8c98495a0",
          "message": "chore(deps): bump tracing-subscriber from 0.3.22 to 0.3.23 in /loom-tests (#114)\n\nBumps [tracing-subscriber](https://github.com/tokio-rs/tracing) from\n0.3.22 to 0.3.23.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/tokio-rs/tracing/releases\">tracing-subscriber's\nreleases</a>.</em></p>\n<blockquote>\n<h2>tracing-subscriber 0.3.23</h2>\n<h3>Fixed</h3>\n<ul>\n<li>Allow ansi sanitization to be disabled (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3484\">#3484</a>)</li>\n</ul>\n<p><a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3484\">#3484</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/pull/3484\">tokio-rs/tracing#3484</a></p>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/54ede4d5d85a536aed5485c5213011d9ec961935\"><code>54ede4d</code></a>\nchore: prepare tracing-subscriber 0.3.23 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3490\">#3490</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/37558d5f26340e999089bf3a680a800435332312\"><code>37558d5</code></a>\nsubscriber: allow ansi sanitization to be disabled (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3484\">#3484</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/efc690fa6bd1d9c3a57528b9bc8ac80504a7a6ed\"><code>efc690f</code></a>\ncore: add missing const (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3449\">#3449</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/0c32367cf9df27e750c4c81803de62a4e64e2ef1\"><code>0c32367</code></a>\ncore: Use const initializers instead of <code>once_cell</code></li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/9feb241133e55e70c7d4399689b8ef72f71d070f\"><code>9feb241</code></a>\ndocs: add arcswap reload crate to related (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3442\">#3442</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/2d55f6faf9be83e7e4634129fb96813241aac2b8\"><code>2d55f6f</code></a>\nchore: prepare tracing 0.1.44 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3439\">#3439</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/10a9e838a35e6ded79d66af246be2ee05417136d\"><code>10a9e83</code></a>\nchore: prepare tracing-core 0.1.36 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/ee82cf92a8c750f98cfb7a417cc8defb37e26a00\"><code>ee82cf9</code></a>\ntracing: fix record_all panic (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/9978c3663bcd58de14b3cf089ad24cb63d00a922\"><code>9978c36</code></a>\nchore: prepare tracing-mock 0.1.0-beta.3 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3429\">#3429</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/tokio-rs/tracing/compare/tracing-subscriber-0.3.22...tracing-subscriber-0.3.23\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=tracing-subscriber&package-manager=cargo&previous-version=0.3.22&new-version=0.3.23)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-20T05:23:48-07:00",
          "tree_id": "539f491bfe82e84b9fc5addf949a64af993f4017",
          "url": "https://github.com/wallstop/fortress-rollback/commit/dd673ceae4b6e245af0e426adfd251f8c98495a0"
        },
        "date": 1774009726971,
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
            "range": "± 0",
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
            "value": 448,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 675,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1024,
            "range": "± 39",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102772,
            "range": "± 715",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27134,
            "range": "± 1105",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
            "range": "± 65",
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
          "id": "9b8b6bd4b4110795cf49fb235b2e4c2b69779b06",
          "message": "Deterministic test infrastructure with injectable clocks and project reorganization (#119)\n\n## Summary\n\nIntroduce an injectable `ClockFn` abstraction and in-memory\n`ChannelSocket` transport to enable fully deterministic, fast, and\nplatform-independent integration tests — replacing real UDP sockets,\n`thread::sleep()`, and `#[serial]` test ordering throughout the test\nsuite. Also reorganizes scripts and LLM skills into domain-based\nsubdirectories.\n\n### Clock injection & deterministic time control\n- Add `ClockFn` type alias (`Arc<dyn Fn() -> Instant + Send + Sync>`)\nand `ProtocolConfig::clock` field for injectable time sources\n- Add `ChaosSocket::with_clock()` builder method for deterministic\nchaos/network-condition testing\n- Replace all `Instant::now()` calls in `Protocol` and `ChaosSocket`\nwith the injected clock (falling back to system time)\n- **Breaking:** `ProtocolConfig` no longer implements `Copy` (due to\n`Arc`-based clock field)\n\n### New test infrastructure\n- `ChannelSocket`: in-memory `NonBlockingSocket` using `mpsc` channels —\neliminates all real UDP I/O from tests\n- `TestClock`: manually-advanceable virtual clock with\n`advance(duration)`, `as_protocol_clock()`, and `as_chaos_clock()`\nhelpers\n- Deterministic test utilities: `synchronize_sessions_deterministic()`,\n`poll_with_advance()`, `run_p2p_frame_advancement_test_deterministic()`\n\n### Test suite migration\n- Converted all resilience, p2p, p2p_enum, and spectator tests from real\nsockets + `thread::sleep` + `#[serial]` to `ChannelSocket` + `TestClock`\n- Removed `serial_test` dependency and all `#[serial]` attributes from\nmigrated tests\n- Removed `#[cfg_attr(miri, ignore)]` from timing-dependent chaos socket\ntests (now virtual-time-based)\n\n### Scripts & skills reorganization\n- Reorganized `scripts/` into `build/`, `ci/`, `docs/`, `verification/`\nsubdirectories\n- Reorganized `.llm/skills/` into 8 category subdirectories\n(rust-language, testing, formal-verification, etc.)\n- Added 8 new workflow skills (code-review, adversarial-review,\ndev-pipeline, investigation, etc.)\n- Added `.llm/design-history/` and `.llm/templates/ask-user-question.md`\n\n### Documentation & CI\n- Updated user guide with custom clock documentation, new config\npresets, and expanded API reference\n- Updated all wiki pages with corrected examples, API signatures, and\nverification status\n- Updated all CI workflows and pre-commit hooks for new script paths\n- Expanded devcontainer with additional VS Code extensions and editor\nsettings\n- Added logo banner SVG\n\n## Test plan\n- [ ] `cargo nextest run --no-capture` — all tests pass with\ndeterministic infrastructure\n- [ ] `cargo clippy --all-targets --features tokio,json` — no warnings\n- [ ] CI workflows resolve scripts at new paths\n- [ ] Pre-commit hooks work with reorganized script layout\n- [ ] Verify no `#[serial]` tests remain in migrated test files\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)",
          "timestamp": "2026-03-20T18:23:48-07:00",
          "tree_id": "0dce9ac3e595e1a3f2fd01090302bb85bc68a6cb",
          "url": "https://github.com/wallstop/fortress-rollback/commit/9b8b6bd4b4110795cf49fb235b2e4c2b69779b06"
        },
        "date": 1774056508166,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 456,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 679,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 996,
            "range": "± 40",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 106651,
            "range": "± 3512",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 41134,
            "range": "± 638",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1243,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
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
          "id": "c5930e1a4004f212b6bb63591f15c3ac9532cbd3",
          "message": "chore(deps): bump z3 from 0.19.13 to 0.19.15 (#118)\n\nBumps [z3](https://github.com/prove-rs/z3.rs) from 0.19.13 to 0.19.15.\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/6d0853207b64aee8efd66c3e297b59f53ccbd5bc\"><code>6d08532</code></a>\nchore(z3): release v0.19.15 (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/521\">#521</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/e7f183c1b4588b74ee5f8ec7161b42bd75dc8b12\"><code>e7f183c</code></a>\ndoc: add missing docs for Boolean operators and pseudo-boolean\nconstraints (#...</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/2e0fd81faa59bc3da0550ff9c8c37af70725d26d\"><code>2e0fd81</code></a>\ndoc: add missing doc comments for Solver::eval() and several Ast impls\n(<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/526\">#526</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/cb655ca82388e3e45fce57a712914453c8b4149e\"><code>cb655ca</code></a>\nadded: Optimize::get_assertions (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/523\">#523</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/b83917b076f18125cadb909d92110eb1d8fe7dfb\"><code>b83917b</code></a>\nfix: {Solver, Optimize}::check_and_get_model no longer take ownership\n(<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/522\">#522</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/0b330e361db8aa78fc3933a2ecda979735e5c40a\"><code>0b330e3</code></a>\nfeat: Implement Optimize Convenience Methods (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/520\">#520</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/c7244999a25ac3178f2d8c2e0abb046574d9a7a8\"><code>c724499</code></a>\nchore: release (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/516\">#516</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/7b22690d42a994601fc43492335316d4a600802d\"><code>7b22690</code></a>\nchore: internal refactoring of datatype builder (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/517\">#517</a>)</li>\n<li><a\nhref=\"https://github.com/prove-rs/z3.rs/commit/70e5dd396f55de899ddd9f4b0dc6786fc312804a\"><code>70e5dd3</code></a>\nUpdate zip dependency to latest (<a\nhref=\"https://redirect.github.com/prove-rs/z3.rs/issues/515\">#515</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/prove-rs/z3.rs/compare/z3-v0.19.13...z3-v0.19.15\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=z3&package-manager=cargo&previous-version=0.19.13&new-version=0.19.15)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-20T18:41:57-07:00",
          "tree_id": "ec1f0a0052117a05c75c4922338655d64e24d36d",
          "url": "https://github.com/wallstop/fortress-rollback/commit/c5930e1a4004f212b6bb63591f15c3ac9532cbd3"
        },
        "date": 1774057584476,
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 431,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 682,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 992,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 104112,
            "range": "± 522",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27495,
            "range": "± 968",
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
          "id": "f520cca23a012371e19651e39b2db5718ff876ba",
          "message": "chore(deps): bump proptest from 1.10.0 to 1.11.0 (#120)\n\nBumps [proptest](https://github.com/proptest-rs/proptest) from 1.10.0 to\n1.11.0.\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/7f1367f9a4dc8440c47b93166a38ed064f63ea8c\"><code>7f1367f</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/641\">#641</a>\nfrom proptest-rs/release-1.11</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/a63bf7eb4e337d76a26a12d3238320acc747551f\"><code>a63bf7e</code></a>\nproptest-state-machine v0.8.0</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/d86e9ff8655cb9833d5e5772195a2485396656f4\"><code>d86e9ff</code></a>\nadd changelog for <a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/640\">#640</a></li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/794031313b4fe42d2c28bad1765a3d22d0b7b8c0\"><code>7940313</code></a>\nproptest v1.11.0</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/3ec998c4d6d9c3992cff9284487914aaeea258e6\"><code>3ec998c</code></a>\nfix <a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/638\">#638</a>\nchangelog</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/8ceb00cfe53f5cf713cd8c007b1c4b9c7d26f401\"><code>8ceb00c</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/639\">#639</a>\nfrom lukoktonos/bits128</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/9c8df1abb945363924bc216dace9e634f6f11ff9\"><code>9c8df1a</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/638\">#638</a>\nfrom folkertdev/f16-support</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/ca9d8e1458518dc22ba1a1b00c92471ba8e6e746\"><code>ca9d8e1</code></a>\nchangelog <a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/638\">#638</a></li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/a39869f5ee5f5aebf43feefd3fd7fec743e230c9\"><code>a39869f</code></a>\nimply f16 feat by unstable</li>\n<li><a\nhref=\"https://github.com/proptest-rs/proptest/commit/85c5ca02764bebeea2cc6261bdf84f9fb9d3eb4c\"><code>85c5ca0</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/proptest-rs/proptest/issues/637\">#637</a>\nfrom folkertdev/min-max-assoc-constants</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/proptest-rs/proptest/compare/v1.10.0...v1.11.0\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=proptest&package-manager=cargo&previous-version=1.10.0&new-version=1.11.0)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-03-26T05:51:11-07:00",
          "tree_id": "29dd15896dbc13dfd90222d6fc2073e68c3fec33",
          "url": "https://github.com/wallstop/fortress-rollback/commit/f520cca23a012371e19651e39b2db5718ff876ba"
        },
        "date": 1774529776210,
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
            "value": 104,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 141,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 559,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 913,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1334,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 95264,
            "range": "± 722",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24315,
            "range": "± 242",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 681,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 871,
            "range": "± 14",
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
          "id": "e499999ee9e94229caf51bac8441709d1a8a444f",
          "message": "Replay integration (#123)\n\n## Description\n\nThis PR delivers a full replay + telemetry feature set and hardens\ndocs/wiki synchronization.\n\nIt solves two core gaps:\n1. No first-class way to record, serialize, and replay real P2P matches\nfor deterministic debugging and validation.\n2. Wiki mirror generation and SYNC-header validation were easier to\ndrift or fail with less-actionable diagnostics.\n\n### What changed\n\n- Added replay domain types and serialization:\n  - `Replay<I>` and `ReplayMetadata`\n- `Replay::to_bytes()`, `Replay::from_bytes()`, `Replay::validate()`,\n`Replay::total_frames()`\n- Added replay playback session:\n  - `ReplaySession<T>` implementing `Session<T>`\n  - `ReplaySession::new()` and `ReplaySession::new_with_validation()`\n- Validation mode emits `SaveGameState` before `AdvanceFrame` and\ncompares checksums frame-by-frame\n- Added replay recording support to P2P sessions:\n  - `SessionBuilder::with_recording(bool)`\n- `P2PSession::is_recording()`, `P2PSession::into_replay()`,\n`P2PSession::take_replay()`\n  - Recording now captures confirmed inputs and recorded checksums\n- Added session telemetry API and built-in collector:\n  - `SessionTelemetry` trait\n- `TelemetryEvent` enum (`Rollback`, `PredictionMiss`,\n`NetworkStatsUpdate`, `FrameAdvance`)\n  - `CollectingTelemetry`\n  - `SessionBuilder::with_telemetry(...)`\n- P2P telemetry emission integrated in rollback, prediction miss,\nnetwork stats polling, and frame advance paths\n- Added sync-layer helper used for telemetry:\n  - `players_with_incorrect_predictions(...)`\n- Expanded public exports and docs:\n  - Re-exports for replay/telemetry in `lib.rs` and prelude\n  - New docs: replay and telemetry guides\n  - Added MkDocs nav entries and wiki mirror pages/sidebar links\n- Hardened docs/wiki synchronization pipeline:\n  - Added pre-commit `sync-wiki` hook before wiki consistency checks\n  - `sync-wiki.py` now:\n    - injects/normalizes reciprocal wiki SYNC headers\n    - validates sidebar coverage before writing files\n    - normalizes generated markdown EOF/newline format deterministically\n- `check-sync-headers.py` now reports case-mismatch hints and\nremediation guidance\n  - Added/expanded script tests for sync behavior and diagnostics\n- Added `.gitignore` entries for local pre-commit/pre-push log artifacts\n\n### Breaking change\n\n- Added `FortressEvent::ReplayDesync { frame, expected_checksum,\nactual_checksum }`.\n- Because `FortressEvent` is not `#[non_exhaustive]`, downstream\nexhaustive `match` statements must add a branch for `ReplayDesync`.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] ✨ New feature (non-breaking change that adds functionality)\n- [x] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --all-targets --features\ntokio,json` with no warnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- Added replay model tests in `src/replay.rs` (serialization roundtrip,\nvalidation invariants, metadata/display, recorder behavior)\n- Added replay session tests in `src/sessions/replay_session.rs`\n(playback flow, completion behavior, validation mode, checksum mismatch\nevent emission)\n- Added event display coverage for `ReplayDesync` in `src/lib.rs`\n- Added builder/session tests in `src/sessions/builder.rs` and\n`src/sessions/p2p_session.rs` for replay-session creation and recording\nAPI behavior\n- Added new script tests in `scripts/tests/test_check_sync_headers.py`\n- Expanded `scripts/tests/test_sync_wiki.py` coverage for SYNC header\ngeneration, sidebar coverage validation, idempotent output\nnormalization, and fail-before-write behavior\n\n**Manual testing performed:**\n\n- Reviewed API docs and migration impact for new replay + telemetry APIs\n- Verified docs navigation additions and wiki mirror wiring in this\nbranch\n- Full local command status (`cargo fmt`, `cargo clippy`, `cargo\nnextest`) intentionally left unchecked in checklist for final author\nconfirmation\n\n## Related Issues\n\n- N/A (no issue links provided in branch metadata)\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->",
          "timestamp": "2026-03-29T16:35:11-07:00",
          "tree_id": "15feb05b7686bdbc232233a67eec695a0138b41b",
          "url": "https://github.com/wallstop/fortress-rollback/commit/e499999ee9e94229caf51bac8441709d1a8a444f"
        },
        "date": 1774827618945,
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
            "value": 170,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 463,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 757,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1097,
            "range": "± 59",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 97303,
            "range": "± 428",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 36703,
            "range": "± 615",
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
            "value": 1607,
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
            "name": "wallstop",
            "username": "wallstop"
          },
          "committer": {
            "email": "wallstop@wallstopstudios.com",
            "name": "wallstop",
            "username": "wallstop"
          },
          "distinct": true,
          "id": "f67e6f55f19ec552a531c2c74a58064eef66ab59",
          "message": "Update version",
          "timestamp": "2026-03-29T16:57:11-07:00",
          "tree_id": "df7a00b8debcc57a8711ec7a6d300b0593730a0b",
          "url": "https://github.com/wallstop/fortress-rollback/commit/f67e6f55f19ec552a531c2c74a58064eef66ab59"
        },
        "date": 1774828900610,
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
            "value": 161,
            "range": "± 6",
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
            "value": 666,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 988,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 103992,
            "range": "± 499",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27246,
            "range": "± 786",
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
      }
    ]
  }
}