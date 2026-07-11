window.BENCHMARK_DATA = {
  "lastUpdate": 1783813589913,
  "repoUrl": "https://github.com/wallstop/fortress-rollback",
  "entries": {
    "Fortress Rollback Benchmarks": [
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
          "id": "6cf1c0f12be4abf3e789c478137f81bd685d6f52",
          "message": "chore(deps): bump the cargo-workspace group with 3 updates (#149)\n\nBumps the cargo-workspace group with 3 updates:\n[tokio](https://github.com/tokio-rs/tokio),\n[clap](https://github.com/clap-rs/clap) and\n[pastey](https://github.com/as1100k/pastey).\n\nUpdates `tokio` from 1.51.1 to 1.52.1\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/tokio-rs/tokio/releases\">tokio's\nreleases</a>.</em></p>\n<blockquote>\n<h2>Tokio v1.52.1</h2>\n<h1>1.52.1 (April 16th, 2026)</h1>\n<h2>Fixed</h2>\n<ul>\n<li>runtime: revert <a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7757\">#7757</a>\nto fix [a regression]<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8056\">#8056</a>\nthat causes <code>spawn_blocking</code> to hang (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8057\">#8057</a>)</li>\n</ul>\n<p><a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7757\">#7757</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/7757\">tokio-rs/tokio#7757</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8056\">#8056</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/8056\">tokio-rs/tokio#8056</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8057\">#8057</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/8057\">tokio-rs/tokio#8057</a></p>\n<h2>Tokio v1.52.0</h2>\n<h1>1.52.0 (April 14th, 2026)</h1>\n<h2>Added</h2>\n<ul>\n<li>io: <code>AioSource::register_borrowed</code> for I/O safety support\n(<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7992\">#7992</a>)</li>\n<li>net: add <code>try_io</code> function to <code>unix::pipe</code>\nsender and receiver types (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8030\">#8030</a>)</li>\n</ul>\n<h2>Added (unstable)</h2>\n<ul>\n<li>runtime: <code>Builder::enable_eager_driver_handoff</code> setting\nenable eager hand off of the I/O and time drivers before polling tasks\n(<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8010\">#8010</a>)</li>\n<li>taskdump: add <code>trace_with()</code> for customized task dumps\n(<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8025\">#8025</a>)</li>\n<li>taskdump: allow <code>impl FnMut()</code> in <code>trace_with</code>\ninstead of just <code>fn()</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8040\">#8040</a>)</li>\n<li>fs: support <code>io_uring</code> in <code>AsyncRead</code> for\n<code>File</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7907\">#7907</a>)</li>\n</ul>\n<h2>Changed</h2>\n<ul>\n<li>runtime: improve <code>spawn_blocking</code> scalability with\nsharded queue (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7757\">#7757</a>)</li>\n<li>runtime: use <code>compare_exchange_weak()</code> in worker queue\n(<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8028\">#8028</a>)</li>\n</ul>\n<h2>Fixed</h2>\n<ul>\n<li>runtime: overflow second half of tasks when local queue is filled\ninstead of first half (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8029\">#8029</a>)</li>\n</ul>\n<h2>Documented</h2>\n<ul>\n<li>docs: fix typo in <code>oneshot::Sender::send</code> docs (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8026\">#8026</a>)</li>\n<li>docs: hide #[tokio::main] attribute in the docs of\n<code>sync::watch</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8035\">#8035</a>)</li>\n<li>net: add docs on <code>ConnectionRefused</code> errors with UDP\nsockets (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7870\">#7870</a>)</li>\n</ul>\n<p><a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7757\">#7757</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/7757\">tokio-rs/tokio#7757</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7870\">#7870</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/7870\">tokio-rs/tokio#7870</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7907\">#7907</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/7907\">tokio-rs/tokio#7907</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7992\">#7992</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/7992\">tokio-rs/tokio#7992</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8010\">#8010</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/8010\">tokio-rs/tokio#8010</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8025\">#8025</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/8025\">tokio-rs/tokio#8025</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8026\">#8026</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/8026\">tokio-rs/tokio#8026</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8028\">#8028</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/8028\">tokio-rs/tokio#8028</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8029\">#8029</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/pull/8029\">tokio-rs/tokio#8029</a></p>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/905c146aeda741ea2202f942a7c3a606dda13da5\"><code>905c146</code></a>\nchore: prepare to release v1.52.1 (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8059\">#8059</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/56aaa43e91c4fbed88f0c2a5b65019ed9a0c3c61\"><code>56aaa43</code></a>\nrt: revert <a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7757\">#7757</a>\nto fix regression in <code>spawn_blocking</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8057\">#8057</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/57ff47ab589bfb4dab6766de78655ffef4fb250b\"><code>57ff47a</code></a>\nci: update <code>trybuild</code> to expect output from rustc 1.95.0 (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8058\">#8058</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/812de3e134888d1d9e7832e4b789d51f6fd2f749\"><code>812de3e</code></a>\nci: bump taiki-e/cache-cargo-install-action from 1 to 3 (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8053\">#8053</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/ba82e73c7b804324c82b6fea6966ca12f55c3826\"><code>ba82e73</code></a>\nci: use Dependabot to keep github actions up to date (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8052\">#8052</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/2e85f9ddf8b47197fa6299cc295f4319fec68e53\"><code>2e85f9d</code></a>\nci: replace cirrus-ci with freebsd-vm (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8041\">#8041</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/a7e1cd8ff8a2012cce500fd7e6ae73400531f46d\"><code>a7e1cd8</code></a>\nci: update GitHub Actions workflows to use latest tool versions (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8047\">#8047</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/5f7be0ac42cb3e1b739da1562f98a797cd55a606\"><code>5f7be0a</code></a>\nchore: perpare 1.52.0 (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8045\">#8045</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/36d12d2686a64b9146c674e02e3cf81d8f87163d\"><code>36d12d2</code></a>\ntaskdump: allow impl FnMut() in taskdumps instead of just fn() (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/8040\">#8040</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tokio/commit/f943312865b9d5007f25d2fd5bd8efa3f89d1541\"><code>f943312</code></a>\nfs: support io-uring in <code>AsyncRead</code> for <code>File</code> (<a\nhref=\"https://redirect.github.com/tokio-rs/tokio/issues/7907\">#7907</a>)</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/tokio-rs/tokio/compare/tokio-1.51.1...tokio-1.52.1\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\nUpdates `clap` from 4.6.0 to 4.6.1\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/releases\">clap's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v4.6.1</h2>\n<h2>[4.6.1] - 2026-04-15</h2>\n<h3>Fixes</h3>\n<ul>\n<li><em>(derive)</em> Ensure rebuilds happen when an read env variable\nis changed</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/clap-rs/clap/blob/master/CHANGELOG.md\">clap's\nchangelog</a>.</em></p>\n<blockquote>\n<h2>[4.6.1] - 2026-04-15</h2>\n<h3>Fixes</h3>\n<ul>\n<li><em>(derive)</em> Ensure rebuilds happen when an read env variable\nis changed</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/14202755e52802a3d294c4ceeadd703d24b21fe6\"><code>1420275</code></a>\nchore: Release</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/d2c817d151db23e0bff70d3df5f9dd9fc311ad5d\"><code>d2c817d</code></a>\ndocs: Update changelog</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/f88c94e53d40c2427450ed65ec025951906eb1d4\"><code>f88c94e</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6341\">#6341</a>\nfrom epage/sep</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/acbb8225054e0a498f6941f278ad0095a893efe8\"><code>acbb822</code></a>\nfix(complete): Reduce risk of conflict with actual subcommands</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/a49fadbf4acf1853f52ae43a445c8f3c81096b01\"><code>a49fadb</code></a>\nrefactor(complete): Pull out subcommand separator</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/ddc008bbbc1924fbda5d6f2c66bcf4d165984977\"><code>ddc008b</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6332\">#6332</a>\nfrom epage/update</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/497dc50aebe9384dc229e1b4e92850306231f9c9\"><code>497dc50</code></a>\nchore: Update compatible dependencies</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/dca2326243615b2375cccb709b19de912910413d\"><code>dca2326</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/clap-rs/clap/issues/6331\">#6331</a>\nfrom clap-rs/renovate/j178-prek-action-2.x</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/54bdaa340ed434535bbd2d95a05b69d8abd2eb34\"><code>54bdaa3</code></a>\nchore(deps): Update j178/prek-action action to v2</li>\n<li><a\nhref=\"https://github.com/clap-rs/clap/commit/f0d30d961d26f8fb636b33242256fca73a717f77\"><code>f0d30d9</code></a>\nchore: Release</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/clap-rs/clap/compare/clap_complete-v4.6.0...clap_complete-v4.6.1\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\nUpdates `pastey` from 0.2.1 to 0.2.2\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/as1100k/pastey/releases\">pastey's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v0.2.2</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Fix Rust 1.56 compatibility: Handle None-delimited groups in replace\nmodifier in <a\nhref=\"https://redirect.github.com/AS1100K/pastey/pull/25\">AS1100K/pastey#25</a></li>\n<li>increase the code coverage by <a\nhref=\"https://github.com/bharatGoswami8\"><code>@​bharatGoswami8</code></a>\nin <a\nhref=\"https://redirect.github.com/AS1100K/pastey/pull/28\">AS1100K/pastey#28</a></li>\n<li>add coverage on CI by <a\nhref=\"https://github.com/bharatGoswami8\"><code>@​bharatGoswami8</code></a>\nin <a\nhref=\"https://redirect.github.com/AS1100K/pastey/pull/30\">AS1100K/pastey#30</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a\nhref=\"https://github.com/bharatGoswami8\"><code>@​bharatGoswami8</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/AS1100K/pastey/pull/28\">AS1100K/pastey#28</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/AS1100K/pastey/blob/master/CHANGELOG.md#022---2026-04-23\"><code>CHANGELOG.md</code></a></p>\n</blockquote>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/AS1100K/pastey/blob/master/CHANGELOG.md\">pastey's\nchangelog</a>.</em></p>\n<blockquote>\n<h2>[0.2.2] - 2026-04-23</h2>\n<h3>Improved</h3>\n<ul>\n<li>Improved Code Coverage <a\nhref=\"https://redirect.github.com/AS1100K/pastey/pull/28\">#28</a>, <a\nhref=\"https://redirect.github.com/AS1100K/pastey/pull/30\">#30</a></li>\n</ul>\n<h3>Fixed</h3>\n<ul>\n<li>Rust 1.56 compatibility: Handling None-delimited groups in replace\nmodifier <a\nhref=\"https://redirect.github.com/AS1100K/pastey/pull/25\">#25</a></li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/AS1100K/pastey/commit/f91b0f4b31ab22e665dedffecfc23410da35b981\"><code>f91b0f4</code></a>\nRELEASE v0.2.2</li>\n<li><a\nhref=\"https://github.com/AS1100K/pastey/commit/93387eb67735614250e7ed97e28e929af07cfaae\"><code>93387eb</code></a>\nadd coverage on CI (<a\nhref=\"https://redirect.github.com/as1100k/pastey/issues/30\">#30</a>)</li>\n<li><a\nhref=\"https://github.com/AS1100K/pastey/commit/113fbc18110446e880d57ab6629b670f8ddfc62c\"><code>113fbc1</code></a>\nincrease the code coverage (<a\nhref=\"https://redirect.github.com/as1100k/pastey/issues/28\">#28</a>)</li>\n<li><a\nhref=\"https://github.com/AS1100K/pastey/commit/436923754b9d9137bef557ccb4c63af67fa9aa2b\"><code>4369237</code></a>\nFix CI Rust 1.56 failure: pin dissimilar to 1.0.10</li>\n<li><a\nhref=\"https://github.com/AS1100K/pastey/commit/6e3ef4a67c14081a1ea6d2be941a725f011976f4\"><code>6e3ef4a</code></a>\nFix Rust 1.56 compatibility: Handle None-delimited groups in replace\nmodifier...</li>\n<li>See full diff in <a\nhref=\"https://github.com/as1100k/pastey/compare/v0.2.1...v0.2.2\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore <dependency name> major version` will close this\ngroup update PR and stop Dependabot creating any more for the specific\ndependency's major version (unless you unignore this specific\ndependency's major version or upgrade to it yourself)\n- `@dependabot ignore <dependency name> minor version` will close this\ngroup update PR and stop Dependabot creating any more for the specific\ndependency's minor version (unless you unignore this specific\ndependency's minor version or upgrade to it yourself)\n- `@dependabot ignore <dependency name>` will close this group update PR\nand stop Dependabot creating any more for the specific dependency\n(unless you unignore this specific dependency or upgrade to it yourself)\n- `@dependabot unignore <dependency name>` will remove all of the ignore\nconditions of the specified dependency\n- `@dependabot unignore <dependency name> <ignore condition>` will\nremove the ignore condition of the specified dependency and ignore\nconditions\n\n\n</details>\n\n---------\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>\nCo-authored-by: copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>\nCo-authored-by: wallstop <1045249+wallstop@users.noreply.github.com>",
          "timestamp": "2026-04-25T08:31:26-07:00",
          "tree_id": "90ebaa1a6175b8aad58dc8dafd963ee198128963",
          "url": "https://github.com/wallstop/fortress-rollback/commit/6cf1c0f12be4abf3e789c478137f81bd685d6f52"
        },
        "date": 1777131387642,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 162,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 448,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 725,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1022,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 103173,
            "range": "± 1692",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27333,
            "range": "± 863",
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
            "range": "± 91",
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
          "id": "70d45e7684fab460c65d6906d616d6e01e53d64e",
          "message": "Improved session behavior (#154)\n\n## Description\n\nAdds user-facing session resilience and runtime tuning improvements for\nP2P play:\n\n- Runtime per-player input delay adjustment via `set_input_delay`,\nincluding supported mid-session increases.\n- Configurable disconnect handling with a new graceful mode\n(`DisconnectBehavior::ContinueWithout`) that allows remaining peers to\ncontinue after a drop.\n- New explicit graceful removal API (`remove_player`) and a new\n`PeerDropped` event for gameplay/UI handling.\n\nThis keeps backward compatibility by preserving halt-on-drop as the\ndefault (`DisconnectBehavior::Halt`).\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n   - No `unwrap()` in production code\n   - No `expect()` in production code\n   - No `panic!()` or `todo!()`\n   - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --all-targets --features\ntokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [ ] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `tests/sessions/input_delay.rs` (new)\n- `tests/sessions/peer_drop.rs` (new)\n- `tests/sessions.rs` (module registration update)\n\n**Manual testing performed:**\n\n- Not performed for this PR description update.\n\n## Related Issues\n\n- None linked.\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Mostly devcontainer + CI workflow changes (tool installation/version\npinning, retries, and link/prose checking) that can affect build\nreliability and developer environment setup, but do not touch runtime\nlibrary logic. Risk is in accidental CI breakage or mismatched tool\nversions across runners/containers.\n> \n> **Overview**\n> **Tooling and CI hardening.** Devcontainer now installs and boots an\noptional Codex CLI (`@openai/codex`) with a persisted `~/.codex` volume\nand non-blocking lifecycle hook (`codex-bootstrap.sh`), and TLA+ tooling\nis version-pinned via `.tla-tools-version` with a shared setup script\nused by both the image and workspace.\n> \n> **More deterministic quality checks.** The `install-cargo-tool` action\nnow verifies cached/install tool versions (new `version-check.sh`) and\nfails on mismatches; `ci-quality` treats `typos` as blocking while\nmaking `cargo-shear`/`cargo-spellcheck` advisory with machine-parsed\noutputs and a clearer summary. Docs CI improves Vale annotation parsing,\nadds external link-check reporting artifacts, and validates Lychee\nconfig before running.\n> \n> **CI reliability tweaks.** Verification/network workflows add\nconcurrency cancellation, Kani version pinning + retry-on-preempt\nbehavior, and cache key fixes (TLA+ cache keyed by\n`.tla-tools-version`). Documentation/examples/changelog are updated to\ndescribe the new runtime input-delay and graceful peer-drop APIs and\ntheir new enum variants.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n4fbc91a7621bb293632de6050f5380ed746bac16. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-05-09T20:40:04-07:00",
          "tree_id": "0f8839daa88c3ff5bf241e41d4c85bfebf556f8a",
          "url": "https://github.com/wallstop/fortress-rollback/commit/70d45e7684fab460c65d6906d616d6e01e53d64e"
        },
        "date": 1778384700162,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
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
            "value": 126,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 175,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 465,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 738,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1088,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 98244,
            "range": "± 1077",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 36634,
            "range": "± 830",
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
            "value": 1601,
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
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f259855ed4a12f45aae368aac2679033d3cbbd8b",
          "message": "Stronger Correctness Around Input Delays and Peer Disconnect (#160)\n\n## Description\n\nStrengthens correctness guarantees around input delay and peer\ndisconnect, with two key behavioral fixes and expanded test coverage.\n\n**Input queue atomicity on gap-fill failure:** `set_frame_delay()`\npreviously left the queue in a partially-written state if the gap-fill\nloop hit an internal invariant violation. It now snapshots all queue\nfields before the loop and fully restores them on failure, so a failed\n`InputQueueGapFillFailed` error leaves the queue unchanged.\n\n**Frozen queues ignore delay changes:** Input queues frozen by\n`ContinueWithout` peer-drop now silently no-op on subsequent\n`set_frame_delay()` calls, preventing mutation of a dropped peer's\nstable simulation input.\n\n**Propagated disconnect knowledge triggers `ContinueWithout`:**\nPreviously `DisconnectBehavior::ContinueWithout` only fired on the\ndetecting peer's automatic timeout. It now also fires when disconnect\nknowledge is propagated from another peer, so all remaining peers apply\ngraceful-drop consistently.\n\n**Fail-closed on freeze failure:** In the `ContinueWithout` auto-drop\npath, if `freeze_player` fails the endpoint is still marked disconnected\nand the session falls back to `Synchronizing` rather than emitting a\nwarning and continuing in a partially-frozen state.\n\n## Type of Change\n\n- [x] =\u001b Bug fix (non-breaking change that fixes an issue)\n- [x] >ê Test (adding or updating tests)\n- [x] =' CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --all-targets --features\ntokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n\n## Testing\n\n**Tests added/modified:**\n\n- `tests/sessions/input_delay.rs` \u0014 new cases covering atomic rollback\nof `set_frame_delay` on failure and frozen-queue no-op behavior\n- `tests/sessions/peer_drop.rs` \u0014 extended coverage of `ContinueWithout`\nwhen disconnect is propagated from a third peer\n- `tests/verification/property.rs` \u0014 new property-based tests for input\nqueue and disconnect invariants\n- `tests/verification/metamorphic.rs` \u0014 metamorphic relation checks for\nqueue state under delay changes\n- `tests/verification/z3.rs` \u0014 additional Z3-backed verification cases\n- `specs/tla/PeerDrop.tla` \u0014 new TLA+ spec modeling graceful peer drop\n- `specs/tla/InputQueue.tla`, `specs/tla/NetworkProtocol.tla` \u0014 extended\nspecs\n\n**Manual testing performed:**\n\n- (None beyond automated tests)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Medium Risk**\n> Touches core rollback/session correctness paths:\n`InputQueue::set_frame_delay` semantics (including frozen queues) and\npeer-drop/disconnect modeling, where subtle state bugs could affect\ndeterminism. Most changes are guarded by expanded fuzz/Kani/TLA+ specs\nand CI checks, reducing regression risk but still warranting careful\nreview.\n> \n> **Overview**\n> Strengthens correctness around runtime input-delay changes and\ngraceful peer drop.\n> \n> `InputQueue::set_frame_delay` is now **transactional** for mid-session\ndelay increases: if the gap-fill replication hits an internal invariant\nfailure, the queue is restored to its pre-call state and returns\n`InputQueueGapFillFailed`. Frozen queues (dropped peers under\n`ContinueWithout`) now treat `set_frame_delay` as a **silent no-op**,\neven for out-of-range delays.\n> \n> Verification and tooling are expanded to lock these behaviors in:\nnew/updated fuzz targets, additional Kani proof harnesses and partition\ntests, extended TLA+ models (`InputQueue` delay/freeze + new `PeerDrop`\nspec, plus protocol invariants), and CI/hook policy updates (new\nversion-sync workflow, docs CI scoping, manual-only slow hooks, new\n`.tla-tools/` guard hook, and a file-scoped `rustfmt` wrapper).\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n361890a2f8005ebe82a5779f35fb1b94bb6cff56. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-05-15T21:47:50-07:00",
          "tree_id": "b84c1c61a96f579cfad43f23bac224e8ef6649fd",
          "url": "https://github.com/wallstop/fortress-rollback/commit/f259855ed4a12f45aae368aac2679033d3cbbd8b"
        },
        "date": 1778907180299,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
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
            "value": 102,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 141,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 566,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 905,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1333,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 95278,
            "range": "± 446",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24720,
            "range": "± 384",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 676,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 869,
            "range": "± 9",
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
          "id": "db2815a303a8b52c6e6b6c4a5f892f770fe2b61a",
          "message": "Bump version (#161)\n\n## Description\n\n<!-- Provide a clear and concise description of your changes. -->\n<!-- What problem does this solve? Why is this change needed? -->\n\n## Type of Change\n\n<!-- Check all that apply -->\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [ ] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [x] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [x] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [x] I have run `cargo fmt && cargo clippy --all-targets --features\ntokio,json` with no warnings\n- [x] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [x] My changes generate no new compiler warnings\n\n## Testing\n\n<!-- Describe how you tested your changes -->\n<!-- Include any relevant details about your testing environment -->\n\n**Tests added/modified:**\n\n- (None)\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n<!-- Link any related issues using GitHub keywords -->\n<!-- Examples: Fixes #123, Closes #456, Relates to #789 -->\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> Low risk: only updates crate version metadata and adds a `0.8.1`\nchangelog entry; no functional code changes are included in this diff.\n> \n> **Overview**\n> Bumps the crate version from `0.8.0` to `0.8.1` in\n`Cargo.toml`/`Cargo.lock` and updates `CHANGELOG.md` to publish the\n`0.8.1` release notes.\n> \n> Updates the changelog compare links so `[Unreleased]` now tracks from\n`v0.8.1` and adds a `[0.8.1]` comparison reference.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\nb63c194e38d9c8d31266ce86ededc683be40addc. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-05-16T08:58:15-07:00",
          "tree_id": "2c6fb1399a229792e6ff630437b1d87880ef3094",
          "url": "https://github.com/wallstop/fortress-rollback/commit/db2815a303a8b52c6e6b6c4a5f892f770fe2b61a"
        },
        "date": 1778947388534,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
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
            "value": 160,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 445,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 707,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1034,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 103877,
            "range": "± 796",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27450,
            "range": "± 846",
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
            "value": 1556,
            "range": "± 99",
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
          "id": "123fa9ed3270ca99ca713877d327a8167dc99fed",
          "message": "Spectator tech (#164)\n\n## Description\n\nThis PR upgrades spectator capabilities with failover, stream delay, and\nrewind support.\n\n- Adds failover spectator startup via\n`SessionBuilder::start_spectator_session_multi(&[T::Address], socket)`.\n- Spectators can connect to multiple hosts at once and continue while at\nleast one host stays connected.\n- Adds `SpectatorConfig::stream_delay` to hold playback behind the live\nedge.\n- Adds `SpectatorConfig::enable_rewind` with seek support through\n`SpectatorSession::seek_to_frame(Frame)`.\n- Adds spectator config validation (`buffer_size > 0` and `stream_delay\n< buffer_size`) and applies it to spectator startup.\n- Improves duplicate frame/input handling and host disconnect behavior\nin multi-host scenarios.\n- Updates user-facing docs and API contracts to document the new\nbehavior and edge cases.\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] ✨ New feature (non-breaking change that adds functionality)\n- [x] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n\t- No `unwrap()` in production code\n\t- No `expect()` in production code\n\t- No `panic!()` or `todo!()`\n\t- All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --all-targets --features\ntokio,json` with no warnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- Updated spectator session coverage in `tests/sessions/spectator.rs`\n- Added/updated spectator config coverage in `src/sessions/config.rs`\n\n**Manual testing performed:**\n\n- (None)\n\n## Related Issues\n\n- (None)\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **High Risk**\n> Breaking public API (spectator config fields, new error/event/reason\nvariants), security-sensitive network decode limits, and broad CI\nworkflow changes that alter release gates and test timing.\n> \n> **Overview**\n> This PR **releases 0.9.0** and documents a large user-facing surface:\n**multi-host spectator failover** (`start_spectator_session_multi`),\n**stream delay** and **rewind/seek**, and **fail-closed handling** when\nredundant hosts disagree (`FortressEvent` /\n`FortressError::SpectatorDivergence`). It also documents **network\nhardening**—fixed-width `Config::Input`, capped RLE/delta/replay\ndecoding, bounded socket receive batches, configurable UDP buffer sizes,\nand clearer endpoint-creation errors.\n> \n> **CI and tooling** change heavily: a shared **`setup-rust-cache`**\ncomposite action (best-effort sccache + `actions/cache`),\n**`--workspace`** for clippy/build aliases, a **`ci-network-nightly`**\nnextest profile and scheduled workflow for ignored real-UDP tests,\n**serial `network-multi-process`** nextest group (180s PR / 720s\nnightly), **enforced semver** on PRs with a failure summary, Docker\nnetem as **continue-on-error** with image pre-pull retries, job\n**timeouts** and **`continue-on-error`** on caches, and a **wiki/\ndry-run sync check**. New **fuzz targets** (protocol input packet,\nreplay decode) and hooks (**`advance_frame` error handling**,\n**`alloc-bound` / `reserve-in-loop`**) tighten safety gates; docs,\nexamples, and LLM guidance are aligned with the new APIs and policies.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\na1952be117d3ecfccd52702b0016c9a6b61fd12d. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-06-04T13:29:30-07:00",
          "tree_id": "28e3c653a7941012372bf546a910da4d08e1b8c9",
          "url": "https://github.com/wallstop/fortress-rollback/commit/123fa9ed3270ca99ca713877d327a8167dc99fed"
        },
        "date": 1780605271660,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
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
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 163,
            "range": "± 3",
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
            "value": 727,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1040,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 133290,
            "range": "± 4926",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 44813,
            "range": "± 305",
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
          "id": "dd5776d0ee3a64ed7e81daf75b27d2354da6135b",
          "message": "feat: Hot Join (#169)\n\n## Description\n\nAdds opt-in hot join support using a reserved-slot model (`hot-join`\nfeature flag).\n\nWith this change, a peer can join an already running session by taking a\nreserved (or gracefully dropped) slot, synchronizing from the host\nsnapshot, and continuing normal rollback play without changing total\nplayer count or input wire width.\n\nThis includes:\n\n- New host/joiner builder APIs for hot-join flows\n- New hot-join session state and events for lifecycle visibility\n- Host-mediated snapshot transfer and apply path (bounded\ndeserialization)\n- Protocol and codec updates for hot-join messages and acknowledgements\n- Integration with existing graceful-peer-drop behavior and slot\nreactivation\n\n## Type of Change\n\n- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [x] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- Added hot-join integration coverage in `tests/sessions/hot_join.rs`\n- Updated related session/macro/determinism test wiring for hot-join\nfeature paths\n\n**Manual testing performed:**\n\n- Not run as part of this PR description draft\n\n## Related Issues\n\n- (None linked in branch commits)\n\n---\n\n<!-- CURSOR_SUMMARY -->\n> [!NOTE]\n> **High Risk**\n> Hot join touches session orchestration, sync/rollback state transfer,\nand network decoding under adversarial input; mistakes can desync or\nstall play, though bounds and ack-gated handshake aim to fail closed.\n> \n> **Overview**\n> Introduces **opt-in hot join** behind the `hot-join` Cargo feature:\npeers fill **reserved or gracefully-dropped** slots without changing\nplayer count or input wire width. Host/joiner **`SessionBuilder`** APIs,\ntunable serve/ack/snapshot wire limits, **`SessionState::HotJoining`**,\n**`JoinRequested` / `PeerJoined`** events,\n**`InvalidRequestKind::PlayerCountMismatch`**, and **`Config::State:\nSerialize + DeserializeOwned`** when the feature is on. Wire support\nadds **`JoinRequest`**, **`StateSnapshot`**, **`StateSnapshotAck`**,\nbounded snapshot decode (**`decode_bounded`**,\n**`MAX_BOUNDED_DECODE_LEN`**), and input-queue **`unfreeze` /\n`reset_to_frame`** for slot reactivation.\n> \n> **CI and docs tooling** moves local link validation to **`python3\nscripts/docs/check-links.py`** (rustdoc intra-doc checks: text-vs-target\nmismatches, `#[cfg(kani)]`-only targets); **`check-links.sh`** is a thin\nwrapper; agent preflight runs link-check on `.md`/`.rs` changes; wiki\ndrift CI prints diagnostics; **`rustdoc-deadlinks`** is dropped in favor\nof strict **`cargo doc`** including a private/feature-gated pass;\n**`ci-rust`** adds a **hot-join** nextest/clippy job. Contributor/LLM\nskill docs and defensive-programming guidance are updated for symmetric\nteardown and single-send hot-join paths.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n60f3ed0cb4bbf390e83d71ff7b1570e5d07a6c00. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-06-05T20:02:22-07:00",
          "tree_id": "74fcf846929cc16356e7ecc827d5b543828baf56",
          "url": "https://github.com/wallstop/fortress-rollback/commit/dd5776d0ee3a64ed7e81daf75b27d2354da6135b"
        },
        "date": 1780715245432,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
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
            "value": 109,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 156,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 445,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 692,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1019,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 136822,
            "range": "± 3167",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45236,
            "range": "± 1530",
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
          "id": "09b8a0962041b2875fe695e5b9eb08efb487727e",
          "message": "feat/bug-fix: Hot-Join + Desync Fixes (#172)\n\n## Description\n\nThis PR delivers three major areas of work:\n\n**1. N≥3-player desync audit and fixes.** A comprehensive adversarial\naudit of desync bugs in sessions with 3 or more players, covering 14\nfindings (F1–F14) and 5 completeness-critic scenarios. All confirmed\nbugs were fixed and regression-tested; contested findings were\narbitrated with red tests before being accepted as NOTABUG. Key fixes\ninclude:\n\n- **Input queue prediction-entry bug (pre-existing, from original GGRS\nport):** A rollback re-simulation whose first input request landed above\na remote queue's missing window would re-enter prediction at the wrong\nframe, silently skipping the misprediction check. This caused confirmed\nstate to permanently diverge with no rollback, no error, and no event —\nand raised false-positive `DesyncDetected` events when detection was on.\n- **Graceful peer drop staggered-detection desync (pre-existing):**\nUnder `DisconnectBehavior::ContinueWithout` with 3+ players, asymmetric\npacket loss could let a survivor run past the mesh-agreed freeze frame\nbefore gossip arrived, permanently embedding a dropped peer's\nunconfirmed high-frame inputs. Fixed by implementing the GGPO-faithful\n`PollNPlayers` gossip-minimum fold in `confirmed_frame()`, a\nconnect-status nudge for input-idle endpoints awaiting mesh agreement,\nand progress-sensitive retransmit suppression.\n- **Sparse save bug:** The checkpoint/save machinery could select a\nstale ring slot for the agreed freeze frame, causing the convergence\nre-roll to use a wrong value.\n- **Spectator host slot overwrite:** A redundant-host failover spectator\ncould overwrite a dropped slot's converged freeze frame with a higher\nvalue from a later host, breaking the global-min guarantee.\n\n**2. Hot-join improvements.** Continued correctness and hardening work\non the `hot-join` feature introduced earlier in this branch: protocol\nand session fixes for slot reactivation, handshake edge cases, and\nmulti-peer interaction.\n\n**3. TLA+ formal verification extended to N=3.** `NetworkProtocol.tla`\nand `NPeerReactivation.tla` were bumped to N=3 and pass TLC with\nnon-vacuous state-space growth (2,804→170,168 and 1,240→9,576 distinct\nstates respectively). `ChecksumExchange.tla` and `TimeSync.tla` were\nrigorously characterized as cannot-cleanly-bump and pinned at N=2 with\ndocumented rationale. The `verify-tla.sh` summary line was fixed to\nreport the true final distinct-state count.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `tests/sessions/desync_harvest.rs` — desync-checksum harvest\narbitration tests (3-peer, deep-prediction, zero false positives proven\nwith neutralization probes)\n- `tests/sessions/hot_join.rs` — hot-join integration coverage, slot\nreactivation, handshake edge cases\n- `tests/sessions/peer_drop.rs` — graceful peer drop scenarios including\nstaggered detection, freeze-frame convergence under asymmetric loss, and\nN≥3 mesh agreement\n- `tests/network/multi_process.rs` — N=3/N=4 real-UDP multi-process\ndriver (previously 2-peer-only)\n- `tests/common/filter_socket.rs` — new `FilterSocket` test harness for\nN≥4 relay-clobber repro\n- `tests/common/reorder_socket.rs` — new `ReorderSocket` test harness\nfor packet-reordering scenarios\n- `tests/verification/z3.rs` — extended Z3 verification coverage\n\n**Manual testing performed:**\n\n- Not run as part of this PR description draft\n\n## Related Issues\n\n- (None linked in branch commits)\n\n---\n\n<!-- CURSOR_SUMMARY -->\n> [!NOTE]\n> **High Risk**\n> Changes core rollback prediction, freeze-frame convergence, and\nmulti-peer disconnect semantics where silent desync was possible;\nhot-join wire additions and changelog-only spectator notes add\nintegration surface area.\n> \n> **Overview**\n> This PR tightens **changelog policy** for `[Unreleased]`: fixes to\nbehavior that already shipped may live under `### Fixed` only when\nprefixed with **`**Pre-existing:**`**, with the hook emitting reviewer\nnotes and tests covering mixed marked/unmarked entries.\n> \n> **Rollback / input-queue correctness** changes how prediction episodes\nstart: they always begin at the queue’s **first missing frame** (not the\nrequested frame), with a Kani proof and regression tests for the F17\n“swallowed window” shape; frame/prediction mismatches now **fail toward\nrollback** instead of skipping comparison. For graceful drop, queues\ngain **`freeze_at`**, **`set_frozen_value_at`** (converge frozen value\nto global-min `F`), and hot-join **`refreeze_with_value`** after abort.\n> \n> **Hot-join wire protocol** adds **`ReactivateSlot`**,\n**`ReactivateSlotAck`**, **`JoinCommitted`**, and **`JoinAborted`**\nmessage types with bounded codec roundtrip/truncation tests.\n**`CHANGELOG.md`** documents large **pre-existing N≥3 desync fixes**\n(prediction entry, `ContinueWithout` gossip-min / nudge), hot-join\nlimits, and expanded spectator failover semantics.\n> \n> **Formal verification**: `ChecksumExchange` is re-modeled for\n**per-(local,remote) pair** verdicts at **N=3** with symmetry and\nin-flight caps; new **`NPeerReactivation`** TLA+ (two-attempt retry /\nabort-restore); `NetworkProtocol` cfg bumped to three peers; CI lists\nupdated.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n6ec2ee8a008d1be581ee5394e786d5de7df12b4b. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-06-11T17:40:28-07:00",
          "tree_id": "eda0e3d4bdcc6dada8f6748b5c9ef921eddb41ee",
          "url": "https://github.com/wallstop/fortress-rollback/commit/09b8a0962041b2875fe695e5b9eb08efb487727e"
        },
        "date": 1781225108469,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
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
            "value": 109,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 156,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 435,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 678,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1016,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 140301,
            "range": "± 2530",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 44742,
            "range": "± 239",
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
            "range": "± 91",
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
          "id": "90b0f1deee669e9751b295ed48d0536f99b5d3b4",
          "message": "bugfix: Hot Join Correctness (#176)\n\n## Description\n\nThis PR tightens correctness for `hot-join` and N-player mesh behavior,\nwith a focus on preventing silent desyncs under real network conditions.\n\nMain user-facing outcomes:\n\n- Improves hot-join reliability for multi-peer sessions\n(reserved-slot/rejoin flows, mesh convergence, and handshake edge\ncases).\n- Fixes a pre-existing input-queue prediction-entry bug that could skip\nmisprediction checks and cause silent divergence.\n- Fixes pre-existing graceful-drop convergence issues in 3+ player\nsessions by using mesh-aware confirmed-frame gating and better status\npropagation/retransmit behavior.\n- Adds/updates regression coverage for hot-join and mesh-fix scenarios.\n\nIn short: this branch makes mid-session joins and peer-drop recovery\nsignificantly safer and more deterministic in N-player games.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [ ] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `tests/sessions/hot_join.rs` — expanded integration coverage for\nmulti-peer hot-join/rejoin and handshake edge cases\n\n**Manual testing performed:**\n\n- Not run as part of this PR description draft\n\n## Related Issues\n\n- (None linked in branch commits)\n\n---\n\n<!-- CURSOR_SUMMARY -->\n> [!NOTE]\n> **High Risk**\n> Changes hot-join session semantics, snapshot wire format, and\nrollback/input presentation for multi-machine meshes; mistakes here\ncause silent desync rather than obvious API breaks.\n> \n> **Overview**\n> **N-peer hot-join (3+ machines)** is no longer rejected at build time.\nServing hosts and mesh joiners are documented and gated with mirrored\nrequirements (`SaveMode::EveryFrame`, zero input delay, local player on\ncoordinator; every-frame saving on N-peer joiners). `StateSnapshot`\ngains **`bridge_inputs`** and **`bridge_statuses`** so the coordinator\ncan ship confirmed inputs and per-slot connection state at snapshot\nframe `S`; empty vs non-empty blobs distinguish 2-peer vs N-peer shapes.\nNew paths **`capture_npeer_snapshot_with_max_wire_bytes`**,\n**`apply_npeer_snapshot`**, and bridge encode/decode simulate the\none-frame bridge to activation `F`, freeze carried-disconnected slots,\nderive bridge `InputStatus` like survivors, and arm a **reactivation\nfloor** on the joining slot so sparse rollbacks replay the original\nbridge presentation.\n> \n> **Wire safety and decoding:** `decode_message` bounds `bridge_inputs`\n/ `bridge_statuses` before reserve; per-player input reconstruction uses\n**`decode_bounded_with_consumed`** instead of unbounded `codec::decode`.\nDocs and CI now steer peer bytes away from generic decode helpers.\n> \n> **Noise / correctness:** `discard_confirmed_frames` on an **empty**\ninput queue (e.g. post hot-join reactivation) is a trace-level no-op\ninstead of an error violation.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n282bf9f388578c5a97256c8d8c1fd8fe63ddd744. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-06-12T11:09:08-07:00",
          "tree_id": "71bda66d613259b3b97467c192979bc86139856c",
          "url": "https://github.com/wallstop/fortress-rollback/commit/90b0f1deee669e9751b295ed48d0536f99b5d3b4"
        },
        "date": 1781288063661,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
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
            "value": 107,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 154,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 437,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 704,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1010,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 136638,
            "range": "± 2942",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45444,
            "range": "± 191",
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
            "value": 1562,
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
          "id": "864308c721354e4f0e8d11db2547df5aa8b6836d",
          "message": "feat: TLA+ Hardening, Minor Bug Fixes (#177)\n\n## Description\n\nThis PR strengthens formal verification coverage for multi-peer rollback\nbehavior and fixes a user-visible warning issue in graceful-drop\nfreezes.\n\nUser-facing changes:\n\n- Fixes spurious warning logs when freezing a queue at `Frame::NULL`\n(expected in reserved-slot hot-join and pre-input drops).\n- Keeps warning behavior for true error cases (non-NULL freeze frame\nmissing from the input ring).\n- Adds targeted regression tests for `freeze_at` warning behavior.\n- Adds new TLA+ models and CI verification coverage for:\n  - freeze convergence across survivors (`FreezeConvergence`)\n- multi-endpoint frame-advantage aggregation\n(`FrameAdvantageAggregation`)\n  - spectator host failover convergence (`SpectatorFailover`)\n- Expands and clarifies existing TLA+ models/docs (`InputQueue`,\n`TimeSync`, `SpectatorSession`) and updates `CHANGELOG.md`.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [x] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [ ] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `src/input_queue/mod.rs` tests:\n  - `freeze_at_with_null_frame_emits_no_violation`\n  - `freeze_at_with_missing_nonnull_frame_still_warns`\n  - `freeze_at_with_present_frame_emits_no_violation`\n- TLA+ verification set extended in `scripts/verification/verify-tla.sh`\nto include:\n  - `SpectatorFailover`\n  - `FreezeConvergence`\n  - `FrameAdvantageAggregation`\n\n**Manual testing performed:**\n\n- Not run as part of this PR description draft\n\n## Related Issues\n\n- None linked in branch commits.\n\n---\n\n<!-- CURSOR_SUMMARY -->\n> [!NOTE]\n> **Low Risk**\n> The runtime change only affects violation logging for an expected\nNULL-freeze path; freeze semantics are unchanged. New TLA+ specs and\ndocs do not alter production code paths.\n> \n> **Overview**\n> Stops **spurious `ViolationSeverity::Warning` logs** when\n`InputQueue::freeze_at` is called with `Frame::NULL` (reserved hot-join\nslots, drops before any confirmed input). Freezing still happens\nunchanged; only telemetry is corrected so it matches the existing\n`set_frozen_value_at` NULL no-op. **Non-NULL** freeze frames that are\nmissing from the ring still warn.\n> \n> Adds **regression tests** that capture `report_violation!` via a\nthread-local tracing subscriber\n(`freeze_at_with_null_frame_emits_no_violation`, missing-frame still\nwarns, happy path silent).\n> \n> Expands **formal verification**: new TLC models `FreezeConvergence`,\n`FrameAdvantageAggregation`, and `SpectatorFailover` (with `.cfg`\nfiles), wired into `verify-tla.sh` and documented in\n`specs/tla/README.md`. **`InputQueue.tla`** now models `freeze_at` /\n`set_frozen_value_at` and frozen-value determinism; companion specs\ncross-link from `TimeSync.tla` and `SpectatorSession.tla`.\n**`CHANGELOG.md`** documents the NULL-freeze behavior.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n0d799e64365e262b47dfd1cd271c11bddf6ee49d. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-06-13T12:50:21-07:00",
          "tree_id": "e54b56957d5a15b4dc24b1231fd1bb1e9b503a19",
          "url": "https://github.com/wallstop/fortress-rollback/commit/864308c721354e4f0e8d11db2547df5aa8b6836d"
        },
        "date": 1781380514820,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
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
            "value": 117,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 159,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 458,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 756,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1091,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 131063,
            "range": "± 804",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 48995,
            "range": "± 664",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1405,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1603,
            "range": "± 6",
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
          "id": "e740b7a433e0ee7bae11f99f2e9aba4816067de7",
          "message": "feat: Peer-Mesh Hardening (#178)\n\n## Description\n\nThis branch hardens peer-controlled decode paths and improves multi-peer\nrollback diagnostics.\nIt adds a per-peer checksum mismatch counter for `P2PSession`, emits a\none-time advisory warning when a peer shows persistent checksum\ndivergence, and keeps that signal advisory rather than auto-ejecting a\npeer.\n\nIt also makes hot-join snapshot decoding reject deeply nested recursive\nstate payloads with a recoverable error instead of risking a stack\noverflow, while keeping the existing bounded-allocation protections.\n\nBeyond the runtime changes, this branch expands N>=3/N>=4 chaos and\npeer-drop coverage and adds a new `DoubleFailureRelay` TLA+ spec plus\ndocumentation that formalize the remaining multi-peer freeze-barrier\nresidual. That spec work proves the mesh-acked-floor fix design and\ndocuments why cheaper cache-only alternatives are unsound; it does not\nship that protocol change yet.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [x] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [x] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n<!-- Please review and check all applicable items -->\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- Added bounded-recursion decode coverage for hot-join snapshot\ndecoding.\n- Expanded in-process chaos coverage for N>=3 multi-peer topologies.\n- Added/expanded peer-drop regressions for checksum-mismatch persistence\nand the N=4 double-failure relay scenario.\n- Added `DoubleFailureRelay` to the TLA+ verification suite and\nrefreshed the verification docs.\n\n**Manual testing performed:**\n\n- None documented in this PR draft.\n\n## Related Issues\n\n- None referenced.\n\n---\n\n<!-- Thank you for contributing to Fortress Rollback! -->\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **High Risk**\n> Touches P2P desync/checksum behavior, peer-controlled hot-join\ndeserialization, and formalizes a deferred multi-peer freeze-barrier\nfix—runtime changes affect session safety paths and hostile-input\nhandling.\n> \n> **Overview**\n> **P2P diagnostics:** Tracks per-remote **checksum mismatch counts** on\nconfirmed frames, exposes them via\n**`P2PSession::peer_checksum_mismatch_count`**, and logs a **one-time\nadvisory WARNING** when a peer crosses an internal threshold—**no\nauto-eject**; apps choose policy from the raw count. Changelog/docs\nclarify **`SyncHealth::DesyncDetected`** handling.\n> \n> **Hot-join decode hardening:** **`codec::decode_bounded`** for\n**`Config::State`** now uses a new **`codec_depth`** serde wrapper so\npeer snapshots nested past **`MAX_DECODE_DEPTH`** fail with **`Err`**\ninstead of risking stack overflow, while keeping the existing byte cap;\ninput decode stays on the fast path.\n> \n> **Zero-panic / lint:** **`#![cfg_attr(not(test))]`** denies\npanic-prone clippy lints on library **`src/`**; **`rle`** and protocol\npaths replace slicing with **`get`**-based access.\n> \n> **Formal methods:** Adds **`DoubleFailureRelay.tla`** (Baseline /\nTombstone / MeshAgree / InheritedFloor configs), registers the passing\n**MeshAgree** model in **`verify-tla.sh`**, and documents why cheaper\ncache-only fixes are unsound—the **mesh-acked-floor** protocol change is\n**spec-only**, not implemented in production.\n> \n> **CI & docs quality:** Workflow **`run`** steps that pipe to **`tee`**\nuse **`set -o pipefail`**; **`check-rust-semantic-claims.py`** is wired\nthrough **`check-doc-claims.sh`**, **agent preflight**, and **ci-docs**\npath filters, with pytest coverage for semantic claims and tee\npipelines.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n8cccba67525283bb192581f249aa81244e680afc. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-06-14T16:58:19-07:00",
          "tree_id": "9c21e6d3cbae852d6b89d6e1db8377a7cbbfb0f1",
          "url": "https://github.com/wallstop/fortress-rollback/commit/e740b7a433e0ee7bae11f99f2e9aba4816067de7"
        },
        "date": 1781481782107,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
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
            "value": 155,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 431,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 696,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1015,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 128604,
            "range": "± 475",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45721,
            "range": "± 216",
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
            "range": "± 87",
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
          "id": "3e8e7eb630f2e69bac731f7398d58ad4c8690193",
          "message": "feat: Protocol Hardening (#179)\n\n## Description\n\nThis PR hardens multi-peer disconnect/drop convergence and stale-signal\nhandling to prevent silent divergence and stuck states, with the biggest\npractical impact in sessions with 3+ peers.\n\nUser-facing outcomes:\n- Fixes edge cases where survivors could confirm past an agreed freeze\npoint and diverge after a peer drop.\n- Tightens stale relay/ack handling so old state cannot incorrectly\nre-open or mis-freeze slots.\n- Adds/extends spectator and hot-join convergence safeguards around\ndrop/reactivation behavior.\n- Expands TLA+ specs/configs for DoubleFailureRelay and async-ack\nscenarios to validate these safety properties.\n- Updates changelog/docs and peer-drop coverage tests.\n\n## Type of Change\n\n- [x] 🐛 Bug fix (non-breaking change that fixes an issue)\n- [ ] ✨ New feature (non-breaking change that adds functionality)\n- [ ] 💥 Breaking change (fix or feature that would cause existing\nfunctionality to change)\n- [x] 📚 Documentation (changes to documentation only)\n- [ ] ♻️ Refactor (code change that neither fixes a bug nor adds a\nfeature)\n- [x] 🧪 Test (adding or updating tests)\n- [ ] 🔧 CI/Build (changes to CI configuration or build process)\n\n## Checklist\n\n### Required\n\n- [ ] I have read the [CONTRIBUTING guide](../docs/contributing.md)\n- [ ] I have followed the **zero-panic policy**:\n  - No `unwrap()` in production code\n  - No `expect()` in production code\n  - No `panic!()` or `todo!()`\n  - All fallible operations return `Result`\n- [ ] I have added tests that prove my fix is effective or my feature\nworks\n- [ ] I have run `cargo fmt && cargo clippy --workspace --all-targets\n--features tokio,json` with no warnings\n- [ ] I have run `cargo nextest run` and all tests pass\n\n### If Applicable\n\n- [x] I have updated the documentation accordingly\n- [x] I have added an entry to `CHANGELOG.md` for user-facing changes\n- [ ] I have updated relevant examples in the `examples/` directory\n- [ ] My changes generate no new compiler warnings\n\n## Testing\n\n**Tests added/modified:**\n\n- `tests/sessions/peer_drop.rs` (peer-drop convergence scenarios)\n\n**Manual testing performed:**\n\n- TLA+ spec/config scenario expansion under\n`specs/tla/DoubleFailureRelay*`\n- (No additional manual runtime testing documented in this draft)\n\n## Related Issues\n\n- (None linked)\n\n---\n\n<!-- CURSOR_SUMMARY -->\n> [!NOTE]\n> **Medium Risk**\n> CI and hook changes are low risk, but the changelog documents a\nwire-format addition (`pessimistic_floor`) and session compatibility\nrequirements that are security/correctness-critical if the corresponding\nRust changes ship in the same release.\n> \n> **Overview**\n> **Network CI** is re-tiered so per-PR runs stay **loss-free and\nnon-retried**: `FORTRESS_NETWORK_TIER=smoke` blocks packet-loss\n`multi_process` scenarios (retries removed on that step), Docker\n`--quick` drops lossy netem, and nextest slow timeouts are documented\nagainst the **macOS-scaled harness ceiling** (180s PR / 960s nightly).\nNightly jobs set `FORTRESS_NETWORK_TIER=nightly`, raise job timeout to\n240m, and add a **Docker `--all` netem** lane.\n> \n> **Enforcement** adds `check-network-timing-invariants.py` (nextest vs\n`multi_process.rs` constants, no direct `wait_for_peer()`, protocol\ntests use virtual clocks), wired into pre-commit and agent-preflight.\n> \n> **Docs/verification**: new `check-tla-config-consistency.py` (FIX_MODE\nset vs `.cfg` vs README counts) in `ci-docs`; TLA verify drops\n`--fail-fast`, uses **multi-worker TLC**, per-spec `timeout`, and\nstricter pass detection; Miri drops the **s390x cross-target** install\nin favor of host-native checks plus `test_wire_endianness.py`. Preflight\ngains **typos**, **tomllib→tomli fallback**, and expanded path triggers.\n> \n> **CHANGELOG** (in this diff) records user-visible protocol narrative:\n**`pessimistic_floor` on input packets** (wire-compatible session\nrequired), spectator **connect-status epoch** for drop/reactivation\nordering, hot-join **per-endpoint era `magic`**, and narrowed\ndouble-failure-relay residuals; `DoubleFailureRelay.cfg` adds\n`EPOCH_MAX` / `COLD_CACHE`.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\n536cfa0d852341e43c573399d915f3caf69e7614. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->",
          "timestamp": "2026-06-18T13:44:45-07:00",
          "tree_id": "44f5b8aed317e1ab76f244d71b6a7bb9f5341259",
          "url": "https://github.com/wallstop/fortress-rollback/commit/3e8e7eb630f2e69bac731f7398d58ad4c8690193"
        },
        "date": 1781815770462,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
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
            "value": 107,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 154,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 445,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 701,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1030,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 132436,
            "range": "± 2883",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45622,
            "range": "± 439",
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
          "id": "ae75c3fcdf2b292d3068e30cdaa684a426bdcef9",
          "message": "chore(ci): bump actions/checkout from 6 to 7 in the github-actions-all group (#181)\n\nBumps the github-actions-all group with 1 update:\n[actions/checkout](https://github.com/actions/checkout).\n\nUpdates `actions/checkout` from 6 to 7\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/actions/checkout/releases\">actions/checkout's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v7.0.0</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>block checking out fork pr for pull_request_target and workflow_run\nby <a href=\"https://github.com/aiqiaoy\"><code>@​aiqiaoy</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2454\">actions/checkout#2454</a></li>\n<li>Bump actions/publish-immutable-action from 0.0.3 to 0.0.4 in the\nminor-actions-dependencies group across 1 directory by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a>[bot]\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2458\">actions/checkout#2458</a></li>\n<li>Bump flatted from 3.3.1 to 3.4.2 by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a>[bot]\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2460\">actions/checkout#2460</a></li>\n<li>Bump js-yaml from 4.1.0 to 4.2.0 by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a>[bot]\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2461\">actions/checkout#2461</a></li>\n<li>Bump <code>@​actions/core</code> and\n<code>@​actions/tool-cache</code> and Remove uuid by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a>[bot]\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2459\">actions/checkout#2459</a></li>\n<li>upgrade module to esm and update dependencies by <a\nhref=\"https://github.com/aiqiaoy\"><code>@​aiqiaoy</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2463\">actions/checkout#2463</a></li>\n<li>Bump the minor-npm-dependencies group across 1 directory with 3\nupdates by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a>[bot]\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2462\">actions/checkout#2462</a></li>\n<li>getting ready for checkout v7 release by <a\nhref=\"https://github.com/aiqiaoy\"><code>@​aiqiaoy</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2464\">actions/checkout#2464</a></li>\n<li>update error wording by <a\nhref=\"https://github.com/aiqiaoy\"><code>@​aiqiaoy</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2467\">actions/checkout#2467</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/aiqiaoy\"><code>@​aiqiaoy</code></a> made\ntheir first contribution in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2454\">actions/checkout#2454</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/checkout/compare/v6.0.3...v7.0.0\">https://github.com/actions/checkout/compare/v6.0.3...v7.0.0</a></p>\n<h2>v6.0.3</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update changelog by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2357\">actions/checkout#2357</a></li>\n<li>fix: expand merge commit SHA regex and add SHA-256 test cases by <a\nhref=\"https://github.com/yaananth\"><code>@​yaananth</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2414\">actions/checkout#2414</a></li>\n<li>Fix checkout init for SHA-256 repositories by <a\nhref=\"https://github.com/yaananth\"><code>@​yaananth</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2439\">actions/checkout#2439</a></li>\n<li>Update changelog for v6.0.3 by <a\nhref=\"https://github.com/yaananth\"><code>@​yaananth</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2446\">actions/checkout#2446</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/yaananth\"><code>@​yaananth</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2414\">actions/checkout#2414</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/checkout/compare/v6...v6.0.3\">https://github.com/actions/checkout/compare/v6...v6.0.3</a></p>\n<h2>v6.0.2</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Add orchestration_id to git user-agent when ACTIONS_ORCHESTRATION_ID\nis set by <a\nhref=\"https://github.com/TingluoHuang\"><code>@​TingluoHuang</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2355\">actions/checkout#2355</a></li>\n<li>Fix tag handling: preserve annotations and explicit fetch-tags by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2356\">actions/checkout#2356</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/checkout/compare/v6.0.1...v6.0.2\">https://github.com/actions/checkout/compare/v6.0.1...v6.0.2</a></p>\n<h2>v6.0.1</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update all references from v5 and v4 to v6 by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2314\">actions/checkout#2314</a></li>\n<li>Add worktree support for persist-credentials includeIf by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2327\">actions/checkout#2327</a></li>\n<li>Clarify v6 README by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2328\">actions/checkout#2328</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/checkout/compare/v6...v6.0.1\">https://github.com/actions/checkout/compare/v6...v6.0.1</a></p>\n</blockquote>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/actions/checkout/blob/main/CHANGELOG.md\">actions/checkout's\nchangelog</a>.</em></p>\n<blockquote>\n<h1>Changelog</h1>\n<h2>v7.0.0</h2>\n<ul>\n<li>Block checking out fork PR for pull_request_target and workflow_run\nby <a href=\"https://github.com/aiqiaoy\"><code>@​aiqiaoy</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2454\">actions/checkout#2454</a></li>\n<li>Bump actions/publish-immutable-action from 0.0.3 to 0.0.4 in the\nminor-actions-dependencies group across 1 directory by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a>[bot]\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2458\">actions/checkout#2458</a></li>\n<li>Bump flatted from 3.3.1 to 3.4.2 by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a>[bot]\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2460\">actions/checkout#2460</a></li>\n<li>Bump js-yaml from 4.1.0 to 4.2.0 by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a>[bot]\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2461\">actions/checkout#2461</a></li>\n<li>Bump <code>@​actions/core</code> and\n<code>@​actions/tool-cache</code> and Remove uuid by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a>[bot]\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2459\">actions/checkout#2459</a></li>\n<li>upgrade module to esm and update dependencies by <a\nhref=\"https://github.com/aiqiaoy\"><code>@​aiqiaoy</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2463\">actions/checkout#2463</a></li>\n<li>Bump the minor-npm-dependencies group across 1 directory with 3\nupdates by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a>[bot]\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2462\">actions/checkout#2462</a></li>\n</ul>\n<h2>v6.0.3</h2>\n<ul>\n<li>Fix checkout init for SHA-256 repositories by <a\nhref=\"https://github.com/yaananth\"><code>@​yaananth</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2439\">actions/checkout#2439</a></li>\n<li>fix: expand merge commit SHA regex and add SHA-256 test cases by <a\nhref=\"https://github.com/yaananth\"><code>@​yaananth</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2414\">actions/checkout#2414</a></li>\n</ul>\n<h2>v6.0.2</h2>\n<ul>\n<li>Fix tag handling: preserve annotations and explicit fetch-tags by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2356\">actions/checkout#2356</a></li>\n</ul>\n<h2>v6.0.1</h2>\n<ul>\n<li>Add worktree support for persist-credentials includeIf by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2327\">actions/checkout#2327</a></li>\n</ul>\n<h2>v6.0.0</h2>\n<ul>\n<li>Persist creds to a separate file by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2286\">actions/checkout#2286</a></li>\n<li>Update README to include Node.js 24 support details and requirements\nby <a href=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2248\">actions/checkout#2248</a></li>\n</ul>\n<h2>v5.0.1</h2>\n<ul>\n<li>Port v6 cleanup to v5 by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2301\">actions/checkout#2301</a></li>\n</ul>\n<h2>v5.0.0</h2>\n<ul>\n<li>Update actions checkout to use node 24 by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2226\">actions/checkout#2226</a></li>\n</ul>\n<h2>v4.3.1</h2>\n<ul>\n<li>Port v6 cleanup to v4 by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2305\">actions/checkout#2305</a></li>\n</ul>\n<h2>v4.3.0</h2>\n<ul>\n<li>docs: update README.md by <a\nhref=\"https://github.com/motss\"><code>@​motss</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1971\">actions/checkout#1971</a></li>\n<li>Add internal repos for checking out multiple repositories by <a\nhref=\"https://github.com/mouismail\"><code>@​mouismail</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1977\">actions/checkout#1977</a></li>\n<li>Documentation update - add recommended permissions to Readme by <a\nhref=\"https://github.com/benwells\"><code>@​benwells</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2043\">actions/checkout#2043</a></li>\n<li>Adjust positioning of user email note and permissions heading by <a\nhref=\"https://github.com/joshmgross\"><code>@​joshmgross</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2044\">actions/checkout#2044</a></li>\n<li>Update README.md by <a\nhref=\"https://github.com/nebuk89\"><code>@​nebuk89</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2194\">actions/checkout#2194</a></li>\n<li>Update CODEOWNERS for actions by <a\nhref=\"https://github.com/TingluoHuang\"><code>@​TingluoHuang</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2224\">actions/checkout#2224</a></li>\n<li>Update package dependencies by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2236\">actions/checkout#2236</a></li>\n</ul>\n<h2>v4.2.2</h2>\n<ul>\n<li><code>url-helper.ts</code> now leverages well-known environment\nvariables by <a href=\"https://github.com/jww3\"><code>@​jww3</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1941\">actions/checkout#1941</a></li>\n<li>Expand unit test coverage for <code>isGhes</code> by <a\nhref=\"https://github.com/jww3\"><code>@​jww3</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1946\">actions/checkout#1946</a></li>\n</ul>\n<h2>v4.2.1</h2>\n<ul>\n<li>Check out other refs/* by commit if provided, fall back to ref by <a\nhref=\"https://github.com/orhantoy\"><code>@​orhantoy</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1924\">actions/checkout#1924</a></li>\n</ul>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0\"><code>9c091bb</code></a>\nupdate error wording (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2467\">#2467</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/1044a6dea927916f2c38ba5aeffbc0a847b1221a\"><code>1044a6d</code></a>\ngetting ready for checkout v7 release (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2464\">#2464</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/f0282184c7ce73ab54c7e4ab5a617122602e575f\"><code>f028218</code></a>\nBump the minor-npm-dependencies group across 1 directory with 3 updates\n(<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2462\">#2462</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/d914b262ffc244530a203ab40decab34c3abf34d\"><code>d914b26</code></a>\nupgrade module to esm and update dependencies (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2463\">#2463</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/537c7ef99cef6e5ddb5e7ff5d16d14510503801d\"><code>537c7ef</code></a>\nBump <code>@​actions/core</code> and <code>@​actions/tool-cache</code>\nand Remove uuid (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2459\">#2459</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/130a169078a413d3a5246a393625e8e742f387f6\"><code>130a169</code></a>\nBump js-yaml from 4.1.0 to 4.2.0 (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2461\">#2461</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/7d09575332117a40b46e5e020664df234cd416f3\"><code>7d09575</code></a>\nBump flatted from 3.3.1 to 3.4.2 (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2460\">#2460</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/0f9f3aa320cb53abeb534aeb54048075d9697a0e\"><code>0f9f3aa</code></a>\nBump actions/publish-immutable-action (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2458\">#2458</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/f9e715a95fcd1f9253f77dd28f11e88d2d6460c7\"><code>f9e715a</code></a>\nblock checking out fork pr for pull_request_target and workflow_run (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2454\">#2454</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/actions/checkout/compare/v6...v7\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=actions/checkout&package-manager=github_actions&previous-version=6&new-version=7)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore <dependency name> major version` will close this\ngroup update PR and stop Dependabot creating any more for the specific\ndependency's major version (unless you unignore this specific\ndependency's major version or upgrade to it yourself)\n- `@dependabot ignore <dependency name> minor version` will close this\ngroup update PR and stop Dependabot creating any more for the specific\ndependency's minor version (unless you unignore this specific\ndependency's minor version or upgrade to it yourself)\n- `@dependabot ignore <dependency name>` will close this group update PR\nand stop Dependabot creating any more for the specific dependency\n(unless you unignore this specific dependency or upgrade to it yourself)\n- `@dependabot unignore <dependency name>` will remove all of the ignore\nconditions of the specified dependency\n- `@dependabot unignore <dependency name> <ignore condition>` will\nremove the ignore condition of the specified dependency and ignore\nconditions\n\n\n</details>\n\n<!-- CURSOR_SUMMARY -->\n---\n\n> [!NOTE]\n> **Low Risk**\n> CI-only dependency pin bump with no application code changes; main\nnuance is checkout v7’s stricter fork-PR rules for certain event types,\nwhich should not affect workflows that already checkout trusted refs.\n> \n> **Overview**\n> Updates every GitHub Actions workflow to use **`actions/checkout@v7`**\ninstead of **`actions/checkout@v6`**. The change is mechanical across\nthe CI, docs, publish, wiki, devcontainer, and Dependabot auto-merge\npipelines—no step inputs, `ref`, or `fetch-depth` settings were altered.\n> \n> **`checkout` v7** brings dependency and runtime updates (including\nESM) and **tightens checkout behavior** for `pull_request_target` and\n`workflow_run` when a fork PR would be checked out. This repo’s\n`dependabot-auto-merge` workflow already checks out the **trusted base\nref** (`pull_request.base.sha`), which aligns with that stricter model.\n> \n> <sup>Reviewed by [Cursor Bugbot](https://cursor.com/bugbot) for commit\ncdec787e4e1c513a61ecf674215709a02f4d862b. Bugbot is set up for automated\ncode reviews on this repo. Configure\n[here](https://www.cursor.com/dashboard/bugbot).</sup>\n<!-- /CURSOR_SUMMARY -->\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2026-06-19T21:49:00-07:00",
          "tree_id": "ecad66e10d260943a3e1b2627e807213c0cffa1b",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ae75c3fcdf2b292d3068e30cdaa684a426bdcef9"
        },
        "date": 1781931248027,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
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
            "value": 107,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 154,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 450,
            "range": "± 30",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 712,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1027,
            "range": "± 61",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 129189,
            "range": "± 4486",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 45459,
            "range": "± 253",
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
            "value": 1557,
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
      }
    ]
  }
}