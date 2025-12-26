window.BENCHMARK_DATA = {
  "lastUpdate": 1766781005000,
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
          "id": "998b733914a43a0e3a6d01f9d5c02c7b5c8103c1",
          "message": "Fix benchmark yaml (#4)",
          "timestamp": "2025-12-18T17:55:33-08:00",
          "tree_id": "0f24067a4fc76b35e2bd4ec140265aa5109c9403",
          "url": "https://github.com/wallstop/fortress-rollback/commit/998b733914a43a0e3a6d01f9d5c02c7b5c8103c1"
        },
        "date": 1766109635922,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 163,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 546,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 820,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1126,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 140,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 28,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 4,
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
          "id": "80025388f7e4932a8f6d1fd3971ff13a7f6b74cb",
          "message": "CI/CD cleanup, add llms.txt, add depndabot (#5)",
          "timestamp": "2025-12-18T18:41:37-08:00",
          "tree_id": "4ee6554727615a22708a67d25efa3aaff301a3a4",
          "url": "https://github.com/wallstop/fortress-rollback/commit/80025388f7e4932a8f6d1fd3971ff13a7f6b74cb"
        },
        "date": 1766112381347,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 106,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 160,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 544,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 799,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1172,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 128,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 4,
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
          "id": "80025388f7e4932a8f6d1fd3971ff13a7f6b74cb",
          "message": "CI/CD cleanup, add llms.txt, add depndabot (#5)",
          "timestamp": "2025-12-18T18:41:37-08:00",
          "tree_id": "4ee6554727615a22708a67d25efa3aaff301a3a4",
          "url": "https://github.com/wallstop/fortress-rollback/commit/80025388f7e4932a8f6d1fd3971ff13a7f6b74cb"
        },
        "date": 1766112388314,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 97,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 144,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 695,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 1004,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1452,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 115,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 23,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
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
          "id": "450135070358537aa546b659b5631fde32186e2c",
          "message": "CI/CD fixes (#13)",
          "timestamp": "2025-12-18T20:57:49-08:00",
          "tree_id": "d1668a276116e3a9cfc0d0e94fcaf58b243eb5fe",
          "url": "https://github.com/wallstop/fortress-rollback/commit/450135070358537aa546b659b5631fde32186e2c"
        },
        "date": 1766120552511,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 106,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 160,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 533,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 802,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1153,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 118299,
            "range": "± 1481",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25445,
            "range": "± 210",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11554,
            "range": "± 158",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
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
          "id": "450135070358537aa546b659b5631fde32186e2c",
          "message": "CI/CD fixes (#13)",
          "timestamp": "2025-12-18T20:57:49-08:00",
          "tree_id": "d1668a276116e3a9cfc0d0e94fcaf58b243eb5fe",
          "url": "https://github.com/wallstop/fortress-rollback/commit/450135070358537aa546b659b5631fde32186e2c"
        },
        "date": 1766120553690,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 161,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 532,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 834,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1180,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 118248,
            "range": "± 1757",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25429,
            "range": "± 368",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11425,
            "range": "± 153",
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
          "id": "4b9cf3b484fdf2a451e4a57f38733752e025ad0a",
          "message": "chore(ci): bump actions/upload-artifact from 4 to 6 (#9)\n\nBumps\n[actions/upload-artifact](https://github.com/actions/upload-artifact)\nfrom 4 to 6.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/actions/upload-artifact/releases\">actions/upload-artifact's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v6.0.0</h2>\n<h2>v6 - What's new</h2>\n<blockquote>\n<p>[!IMPORTANT]\nactions/upload-artifact@v6 now runs on Node.js 24 (<code>runs.using:\nnode24</code>) and requires a minimum Actions Runner version of 2.327.1.\nIf you are using self-hosted runners, ensure they are updated before\nupgrading.</p>\n</blockquote>\n<h3>Node.js 24</h3>\n<p>This release updates the runtime to Node.js 24. v5 had preliminary\nsupport for Node.js 24, however this action was by default still running\non Node.js 20. Now this action by default will run on Node.js 24.</p>\n<h2>What's Changed</h2>\n<ul>\n<li>Upload Artifact Node 24 support by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/719\">actions/upload-artifact#719</a></li>\n<li>fix: update <code>@​actions/artifact</code> for Node.js 24 punycode\ndeprecation by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/744\">actions/upload-artifact#744</a></li>\n<li>prepare release v6.0.0 for Node.js 24 support by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/745\">actions/upload-artifact#745</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/upload-artifact/compare/v5.0.0...v6.0.0\">https://github.com/actions/upload-artifact/compare/v5.0.0...v6.0.0</a></p>\n<h2>v5.0.0</h2>\n<h2>What's Changed</h2>\n<p><strong>BREAKING CHANGE:</strong> this update supports Node\n<code>v24.x</code>. This is not a breaking change per-se but we're\ntreating it as such.</p>\n<ul>\n<li>Update README.md by <a\nhref=\"https://github.com/GhadimiR\"><code>@​GhadimiR</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/681\">actions/upload-artifact#681</a></li>\n<li>Update README.md by <a\nhref=\"https://github.com/nebuk89\"><code>@​nebuk89</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/712\">actions/upload-artifact#712</a></li>\n<li>Readme: spell out the first use of GHES by <a\nhref=\"https://github.com/danwkennedy\"><code>@​danwkennedy</code></a> in\n<a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/727\">actions/upload-artifact#727</a></li>\n<li>Update GHES guidance to include reference to Node 20 version by <a\nhref=\"https://github.com/patrikpolyak\"><code>@​patrikpolyak</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/725\">actions/upload-artifact#725</a></li>\n<li>Bump <code>@actions/artifact</code> to <code>v4.0.0</code></li>\n<li>Prepare <code>v5.0.0</code> by <a\nhref=\"https://github.com/danwkennedy\"><code>@​danwkennedy</code></a> in\n<a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/734\">actions/upload-artifact#734</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/GhadimiR\"><code>@​GhadimiR</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/681\">actions/upload-artifact#681</a></li>\n<li><a href=\"https://github.com/nebuk89\"><code>@​nebuk89</code></a> made\ntheir first contribution in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/712\">actions/upload-artifact#712</a></li>\n<li><a\nhref=\"https://github.com/danwkennedy\"><code>@​danwkennedy</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/727\">actions/upload-artifact#727</a></li>\n<li><a\nhref=\"https://github.com/patrikpolyak\"><code>@​patrikpolyak</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/725\">actions/upload-artifact#725</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/upload-artifact/compare/v4...v5.0.0\">https://github.com/actions/upload-artifact/compare/v4...v5.0.0</a></p>\n<h2>v4.6.2</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update to use artifact 2.3.2 package &amp; prepare for new\nupload-artifact release by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/685\">actions/upload-artifact#685</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/685\">actions/upload-artifact#685</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/upload-artifact/compare/v4...v4.6.2\">https://github.com/actions/upload-artifact/compare/v4...v4.6.2</a></p>\n<h2>v4.6.1</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update to use artifact 2.2.2 package by <a\nhref=\"https://github.com/yacaovsnc\"><code>@​yacaovsnc</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/673\">actions/upload-artifact#673</a></li>\n</ul>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/b7c566a772e6b6bfb58ed0dc250532a479d7789f\"><code>b7c566a</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/actions/upload-artifact/issues/745\">#745</a>\nfrom actions/upload-artifact-v6-release</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/e516bc8500aaf3d07d591fcd4ae6ab5f9c391d5b\"><code>e516bc8</code></a>\ndocs: correct description of Node.js 24 support in README</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/ddc45ed9bca9b38dbd643978d88e3981cdc91415\"><code>ddc45ed</code></a>\ndocs: update README to correct action name for Node.js 24 support</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/615b319bd27bb32c3d64dca6b6ed6974d5fbe653\"><code>615b319</code></a>\nchore: release v6.0.0 for Node.js 24 support</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/017748b48f8610ca8e6af1222f4a618e84a9c703\"><code>017748b</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/actions/upload-artifact/issues/744\">#744</a>\nfrom actions/fix-storage-blob</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/38d4c7997f5510fcc41fc4aae2a6b97becdbe7fc\"><code>38d4c79</code></a>\nchore: rebuild dist</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/7d27270e0cfd253e666c44abac0711308d2d042f\"><code>7d27270</code></a>\nchore: add missing license cache files for <code>@​actions/core</code>,\n<code>@​actions/io</code>, and mi...</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/5f643d3c9475505ccaf26d686ffbfb71a8387261\"><code>5f643d3</code></a>\nchore: update license files for <code>@​actions/artifact</code><a\nhref=\"https://github.com/5\"><code>@​5</code></a>.0.1 dependencies</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/1df1684032c88614064493e1a0478fcb3583e1d0\"><code>1df1684</code></a>\nchore: update package-lock.json with <code>@​actions/artifact</code><a\nhref=\"https://github.com/5\"><code>@​5</code></a>.0.1</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/b5b1a918401ee270935b6b1d857ae66c85f3be6f\"><code>b5b1a91</code></a>\nfix: update <code>@​actions/artifact</code> to ^5.0.0 for Node.js 24\npunycode fix</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/actions/upload-artifact/compare/v4...v6\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=actions/upload-artifact&package-manager=github_actions&previous-version=4&new-version=6)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2025-12-18T21:14:54-08:00",
          "tree_id": "e9ccab8927a8d099a09c6b074bf0889bcdd27bee",
          "url": "https://github.com/wallstop/fortress-rollback/commit/4b9cf3b484fdf2a451e4a57f38733752e025ad0a"
        },
        "date": 1766121573462,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 148,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 673,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 988,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1449,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 113877,
            "range": "± 1752",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 23487,
            "range": "± 289",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 10667,
            "range": "± 342",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 869,
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
          "id": "4b9cf3b484fdf2a451e4a57f38733752e025ad0a",
          "message": "chore(ci): bump actions/upload-artifact from 4 to 6 (#9)\n\nBumps\n[actions/upload-artifact](https://github.com/actions/upload-artifact)\nfrom 4 to 6.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/actions/upload-artifact/releases\">actions/upload-artifact's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v6.0.0</h2>\n<h2>v6 - What's new</h2>\n<blockquote>\n<p>[!IMPORTANT]\nactions/upload-artifact@v6 now runs on Node.js 24 (<code>runs.using:\nnode24</code>) and requires a minimum Actions Runner version of 2.327.1.\nIf you are using self-hosted runners, ensure they are updated before\nupgrading.</p>\n</blockquote>\n<h3>Node.js 24</h3>\n<p>This release updates the runtime to Node.js 24. v5 had preliminary\nsupport for Node.js 24, however this action was by default still running\non Node.js 20. Now this action by default will run on Node.js 24.</p>\n<h2>What's Changed</h2>\n<ul>\n<li>Upload Artifact Node 24 support by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/719\">actions/upload-artifact#719</a></li>\n<li>fix: update <code>@​actions/artifact</code> for Node.js 24 punycode\ndeprecation by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/744\">actions/upload-artifact#744</a></li>\n<li>prepare release v6.0.0 for Node.js 24 support by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/745\">actions/upload-artifact#745</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/upload-artifact/compare/v5.0.0...v6.0.0\">https://github.com/actions/upload-artifact/compare/v5.0.0...v6.0.0</a></p>\n<h2>v5.0.0</h2>\n<h2>What's Changed</h2>\n<p><strong>BREAKING CHANGE:</strong> this update supports Node\n<code>v24.x</code>. This is not a breaking change per-se but we're\ntreating it as such.</p>\n<ul>\n<li>Update README.md by <a\nhref=\"https://github.com/GhadimiR\"><code>@​GhadimiR</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/681\">actions/upload-artifact#681</a></li>\n<li>Update README.md by <a\nhref=\"https://github.com/nebuk89\"><code>@​nebuk89</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/712\">actions/upload-artifact#712</a></li>\n<li>Readme: spell out the first use of GHES by <a\nhref=\"https://github.com/danwkennedy\"><code>@​danwkennedy</code></a> in\n<a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/727\">actions/upload-artifact#727</a></li>\n<li>Update GHES guidance to include reference to Node 20 version by <a\nhref=\"https://github.com/patrikpolyak\"><code>@​patrikpolyak</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/725\">actions/upload-artifact#725</a></li>\n<li>Bump <code>@actions/artifact</code> to <code>v4.0.0</code></li>\n<li>Prepare <code>v5.0.0</code> by <a\nhref=\"https://github.com/danwkennedy\"><code>@​danwkennedy</code></a> in\n<a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/734\">actions/upload-artifact#734</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/GhadimiR\"><code>@​GhadimiR</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/681\">actions/upload-artifact#681</a></li>\n<li><a href=\"https://github.com/nebuk89\"><code>@​nebuk89</code></a> made\ntheir first contribution in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/712\">actions/upload-artifact#712</a></li>\n<li><a\nhref=\"https://github.com/danwkennedy\"><code>@​danwkennedy</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/727\">actions/upload-artifact#727</a></li>\n<li><a\nhref=\"https://github.com/patrikpolyak\"><code>@​patrikpolyak</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/725\">actions/upload-artifact#725</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/upload-artifact/compare/v4...v5.0.0\">https://github.com/actions/upload-artifact/compare/v4...v5.0.0</a></p>\n<h2>v4.6.2</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update to use artifact 2.3.2 package &amp; prepare for new\nupload-artifact release by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/685\">actions/upload-artifact#685</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/685\">actions/upload-artifact#685</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/upload-artifact/compare/v4...v4.6.2\">https://github.com/actions/upload-artifact/compare/v4...v4.6.2</a></p>\n<h2>v4.6.1</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update to use artifact 2.2.2 package by <a\nhref=\"https://github.com/yacaovsnc\"><code>@​yacaovsnc</code></a> in <a\nhref=\"https://redirect.github.com/actions/upload-artifact/pull/673\">actions/upload-artifact#673</a></li>\n</ul>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/b7c566a772e6b6bfb58ed0dc250532a479d7789f\"><code>b7c566a</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/actions/upload-artifact/issues/745\">#745</a>\nfrom actions/upload-artifact-v6-release</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/e516bc8500aaf3d07d591fcd4ae6ab5f9c391d5b\"><code>e516bc8</code></a>\ndocs: correct description of Node.js 24 support in README</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/ddc45ed9bca9b38dbd643978d88e3981cdc91415\"><code>ddc45ed</code></a>\ndocs: update README to correct action name for Node.js 24 support</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/615b319bd27bb32c3d64dca6b6ed6974d5fbe653\"><code>615b319</code></a>\nchore: release v6.0.0 for Node.js 24 support</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/017748b48f8610ca8e6af1222f4a618e84a9c703\"><code>017748b</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/actions/upload-artifact/issues/744\">#744</a>\nfrom actions/fix-storage-blob</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/38d4c7997f5510fcc41fc4aae2a6b97becdbe7fc\"><code>38d4c79</code></a>\nchore: rebuild dist</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/7d27270e0cfd253e666c44abac0711308d2d042f\"><code>7d27270</code></a>\nchore: add missing license cache files for <code>@​actions/core</code>,\n<code>@​actions/io</code>, and mi...</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/5f643d3c9475505ccaf26d686ffbfb71a8387261\"><code>5f643d3</code></a>\nchore: update license files for <code>@​actions/artifact</code><a\nhref=\"https://github.com/5\"><code>@​5</code></a>.0.1 dependencies</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/1df1684032c88614064493e1a0478fcb3583e1d0\"><code>1df1684</code></a>\nchore: update package-lock.json with <code>@​actions/artifact</code><a\nhref=\"https://github.com/5\"><code>@​5</code></a>.0.1</li>\n<li><a\nhref=\"https://github.com/actions/upload-artifact/commit/b5b1a918401ee270935b6b1d857ae66c85f3be6f\"><code>b5b1a91</code></a>\nfix: update <code>@​actions/artifact</code> to ^5.0.0 for Node.js 24\npunycode fix</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/actions/upload-artifact/compare/v4...v6\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=actions/upload-artifact&package-manager=github_actions&previous-version=4&new-version=6)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2025-12-18T21:14:54-08:00",
          "tree_id": "e9ccab8927a8d099a09c6b074bf0889bcdd27bee",
          "url": "https://github.com/wallstop/fortress-rollback/commit/4b9cf3b484fdf2a451e4a57f38733752e025ad0a"
        },
        "date": 1766121573584,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 147,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 653,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 1015,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1442,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 110762,
            "range": "± 931",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 23433,
            "range": "± 380",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 10660,
            "range": "± 404",
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
          "id": "16edb9ba3f1d0bcf0578b3aa03a59cd0ad34815a",
          "message": "chore(ci): bump actions/cache from 4 to 5 (#11)\n\nBumps [actions/cache](https://github.com/actions/cache) from 4 to 5.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/actions/cache/releases\">actions/cache's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v5.0.0</h2>\n<blockquote>\n<p>[!IMPORTANT]\n<strong><code>actions/cache@v5</code> runs on the Node.js 24 runtime and\nrequires a minimum Actions Runner version of\n<code>2.327.1</code>.</strong></p>\n<p>If you are using self-hosted runners, ensure they are updated before\nupgrading.</p>\n</blockquote>\n<hr />\n<h2>What's Changed</h2>\n<ul>\n<li>Upgrade to use node24 by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1630\">actions/cache#1630</a></li>\n<li>Prepare v5.0.0 release by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1684\">actions/cache#1684</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/cache/compare/v4.3.0...v5.0.0\">https://github.com/actions/cache/compare/v4.3.0...v5.0.0</a></p>\n<h2>v4.3.0</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Add note on runner versions by <a\nhref=\"https://github.com/GhadimiR\"><code>@​GhadimiR</code></a> in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1642\">actions/cache#1642</a></li>\n<li>Prepare <code>v4.3.0</code> release by <a\nhref=\"https://github.com/Link\"><code>@​Link</code></a>- in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1655\">actions/cache#1655</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/GhadimiR\"><code>@​GhadimiR</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1642\">actions/cache#1642</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/cache/compare/v4...v4.3.0\">https://github.com/actions/cache/compare/v4...v4.3.0</a></p>\n<h2>v4.2.4</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update README.md by <a\nhref=\"https://github.com/nebuk89\"><code>@​nebuk89</code></a> in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1620\">actions/cache#1620</a></li>\n<li>Upgrade <code>@actions/cache</code> to <code>4.0.5</code> and move\n<code>@protobuf-ts/plugin</code> to dev depdencies by <a\nhref=\"https://github.com/Link\"><code>@​Link</code></a>- in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1634\">actions/cache#1634</a></li>\n<li>Prepare release <code>4.2.4</code> by <a\nhref=\"https://github.com/Link\"><code>@​Link</code></a>- in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1636\">actions/cache#1636</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/nebuk89\"><code>@​nebuk89</code></a> made\ntheir first contribution in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1620\">actions/cache#1620</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/cache/compare/v4...v4.2.4\">https://github.com/actions/cache/compare/v4...v4.2.4</a></p>\n<h2>v4.2.3</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update to use <code>@​actions/cache</code> 4.0.3 package &amp;\nprepare for new release by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1577\">actions/cache#1577</a>\n(SAS tokens for cache entries are now masked in debug logs)</li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1577\">actions/cache#1577</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/cache/compare/v4.2.2...v4.2.3\">https://github.com/actions/cache/compare/v4.2.2...v4.2.3</a></p>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/actions/cache/blob/main/RELEASES.md\">actions/cache's\nchangelog</a>.</em></p>\n<blockquote>\n<h1>Releases</h1>\n<h2>Changelog</h2>\n<h3>5.0.1</h3>\n<ul>\n<li>Update <code>@azure/storage-blob</code> to <code>^12.29.1</code> via\n<code>@actions/cache@5.0.1</code> <a\nhref=\"https://redirect.github.com/actions/cache/pull/1685\">#1685</a></li>\n</ul>\n<h3>5.0.0</h3>\n<blockquote>\n<p>[!IMPORTANT]\n<code>actions/cache@v5</code> runs on the Node.js 24 runtime and\nrequires a minimum Actions Runner version of <code>2.327.1</code>.\nIf you are using self-hosted runners, ensure they are updated before\nupgrading.</p>\n</blockquote>\n<h3>4.3.0</h3>\n<ul>\n<li>Bump <code>@actions/cache</code> to <a\nhref=\"https://redirect.github.com/actions/toolkit/pull/2132\">v4.1.0</a></li>\n</ul>\n<h3>4.2.4</h3>\n<ul>\n<li>Bump <code>@actions/cache</code> to v4.0.5</li>\n</ul>\n<h3>4.2.3</h3>\n<ul>\n<li>Bump <code>@actions/cache</code> to v4.0.3 (obfuscates SAS token in\ndebug logs for cache entries)</li>\n</ul>\n<h3>4.2.2</h3>\n<ul>\n<li>Bump <code>@actions/cache</code> to v4.0.2</li>\n</ul>\n<h3>4.2.1</h3>\n<ul>\n<li>Bump <code>@actions/cache</code> to v4.0.1</li>\n</ul>\n<h3>4.2.0</h3>\n<p>TLDR; The cache backend service has been rewritten from the ground up\nfor improved performance and reliability. <a\nhref=\"https://github.com/actions/cache\">actions/cache</a> now integrates\nwith the new cache service (v2) APIs.</p>\n<p>The new service will gradually roll out as of <strong>February 1st,\n2025</strong>. The legacy service will also be sunset on the same date.\nChanges in these release are <strong>fully backward\ncompatible</strong>.</p>\n<p><strong>We are deprecating some versions of this action</strong>. We\nrecommend upgrading to version <code>v4</code> or <code>v3</code> as\nsoon as possible before <strong>February 1st, 2025.</strong> (Upgrade\ninstructions below).</p>\n<p>If you are using pinned SHAs, please use the SHAs of versions\n<code>v4.2.0</code> or <code>v3.4.0</code></p>\n<p>If you do not upgrade, all workflow runs using any of the deprecated\n<a href=\"https://github.com/actions/cache\">actions/cache</a> will\nfail.</p>\n<p>Upgrading to the recommended versions will not break your\nworkflows.</p>\n<h3>4.1.2</h3>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/actions/cache/commit/9255dc7a253b0ccc959486e2bca901246202afeb\"><code>9255dc7</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/actions/cache/issues/1686\">#1686</a>\nfrom actions/cache-v5.0.1-release</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/8ff5423e8b66eacab4e638ee52abbd2cb831366a\"><code>8ff5423</code></a>\nchore: release v5.0.1</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/9233019a152bc768059ac1768b8e4403b5da16c1\"><code>9233019</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/actions/cache/issues/1685\">#1685</a>\nfrom salmanmkc/node24-storage-blob-fix</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/b975f2bb844529e1063ad882c609b224bcd66eb6\"><code>b975f2b</code></a>\nfix: add peer property to package-lock.json for dependencies</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/d0a0e1813491d01d574c95f8d189f62622bbb2ae\"><code>d0a0e18</code></a>\nfix: update license files for <code>@​actions/cache</code>,\nfast-xml-parser, and strnum</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/74de208dcfcbe85c0e7154e7b17e4105fe2554ff\"><code>74de208</code></a>\nfix: update <code>@​actions/cache</code> to ^5.0.1 for Node.js 24\npunycode fix</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/ac7f1152ead02e89c14b5456d14ab17591e74cfb\"><code>ac7f115</code></a>\npeer</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/b0f846b50b6061d7a2ca6f1a2fea61d4a65d1a16\"><code>b0f846b</code></a>\nfix: update <code>@​actions/cache</code> with storage-blob fix for\nNode.js 24 punycode depr...</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/a7833574556fa59680c1b7cb190c1735db73ebf0\"><code>a783357</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/actions/cache/issues/1684\">#1684</a>\nfrom actions/prepare-cache-v5-release</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/3bb0d78750a39cefce0c2b5a0a9801052b4359ad\"><code>3bb0d78</code></a>\ndocs: highlight v5 runner requirement in releases</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/actions/cache/compare/v4...v5\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=actions/cache&package-manager=github_actions&previous-version=4&new-version=5)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2025-12-18T21:36:10-08:00",
          "tree_id": "917db7019d27242b8e3e6f6077f43fe6785363ec",
          "url": "https://github.com/wallstop/fortress-rollback/commit/16edb9ba3f1d0bcf0578b3aa03a59cd0ad34815a"
        },
        "date": 1766122849260,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 105,
            "range": "± 2",
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
            "value": 525,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 812,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1169,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 118715,
            "range": "± 1894",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25497,
            "range": "± 3856",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11315,
            "range": "± 164",
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
          "id": "16edb9ba3f1d0bcf0578b3aa03a59cd0ad34815a",
          "message": "chore(ci): bump actions/cache from 4 to 5 (#11)\n\nBumps [actions/cache](https://github.com/actions/cache) from 4 to 5.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/actions/cache/releases\">actions/cache's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v5.0.0</h2>\n<blockquote>\n<p>[!IMPORTANT]\n<strong><code>actions/cache@v5</code> runs on the Node.js 24 runtime and\nrequires a minimum Actions Runner version of\n<code>2.327.1</code>.</strong></p>\n<p>If you are using self-hosted runners, ensure they are updated before\nupgrading.</p>\n</blockquote>\n<hr />\n<h2>What's Changed</h2>\n<ul>\n<li>Upgrade to use node24 by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1630\">actions/cache#1630</a></li>\n<li>Prepare v5.0.0 release by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1684\">actions/cache#1684</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/cache/compare/v4.3.0...v5.0.0\">https://github.com/actions/cache/compare/v4.3.0...v5.0.0</a></p>\n<h2>v4.3.0</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Add note on runner versions by <a\nhref=\"https://github.com/GhadimiR\"><code>@​GhadimiR</code></a> in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1642\">actions/cache#1642</a></li>\n<li>Prepare <code>v4.3.0</code> release by <a\nhref=\"https://github.com/Link\"><code>@​Link</code></a>- in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1655\">actions/cache#1655</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/GhadimiR\"><code>@​GhadimiR</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1642\">actions/cache#1642</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/cache/compare/v4...v4.3.0\">https://github.com/actions/cache/compare/v4...v4.3.0</a></p>\n<h2>v4.2.4</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update README.md by <a\nhref=\"https://github.com/nebuk89\"><code>@​nebuk89</code></a> in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1620\">actions/cache#1620</a></li>\n<li>Upgrade <code>@actions/cache</code> to <code>4.0.5</code> and move\n<code>@protobuf-ts/plugin</code> to dev depdencies by <a\nhref=\"https://github.com/Link\"><code>@​Link</code></a>- in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1634\">actions/cache#1634</a></li>\n<li>Prepare release <code>4.2.4</code> by <a\nhref=\"https://github.com/Link\"><code>@​Link</code></a>- in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1636\">actions/cache#1636</a></li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/nebuk89\"><code>@​nebuk89</code></a> made\ntheir first contribution in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1620\">actions/cache#1620</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/cache/compare/v4...v4.2.4\">https://github.com/actions/cache/compare/v4...v4.2.4</a></p>\n<h2>v4.2.3</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update to use <code>@​actions/cache</code> 4.0.3 package &amp;\nprepare for new release by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1577\">actions/cache#1577</a>\n(SAS tokens for cache entries are now masked in debug logs)</li>\n</ul>\n<h2>New Contributors</h2>\n<ul>\n<li><a href=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a>\nmade their first contribution in <a\nhref=\"https://redirect.github.com/actions/cache/pull/1577\">actions/cache#1577</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/cache/compare/v4.2.2...v4.2.3\">https://github.com/actions/cache/compare/v4.2.2...v4.2.3</a></p>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/actions/cache/blob/main/RELEASES.md\">actions/cache's\nchangelog</a>.</em></p>\n<blockquote>\n<h1>Releases</h1>\n<h2>Changelog</h2>\n<h3>5.0.1</h3>\n<ul>\n<li>Update <code>@azure/storage-blob</code> to <code>^12.29.1</code> via\n<code>@actions/cache@5.0.1</code> <a\nhref=\"https://redirect.github.com/actions/cache/pull/1685\">#1685</a></li>\n</ul>\n<h3>5.0.0</h3>\n<blockquote>\n<p>[!IMPORTANT]\n<code>actions/cache@v5</code> runs on the Node.js 24 runtime and\nrequires a minimum Actions Runner version of <code>2.327.1</code>.\nIf you are using self-hosted runners, ensure they are updated before\nupgrading.</p>\n</blockquote>\n<h3>4.3.0</h3>\n<ul>\n<li>Bump <code>@actions/cache</code> to <a\nhref=\"https://redirect.github.com/actions/toolkit/pull/2132\">v4.1.0</a></li>\n</ul>\n<h3>4.2.4</h3>\n<ul>\n<li>Bump <code>@actions/cache</code> to v4.0.5</li>\n</ul>\n<h3>4.2.3</h3>\n<ul>\n<li>Bump <code>@actions/cache</code> to v4.0.3 (obfuscates SAS token in\ndebug logs for cache entries)</li>\n</ul>\n<h3>4.2.2</h3>\n<ul>\n<li>Bump <code>@actions/cache</code> to v4.0.2</li>\n</ul>\n<h3>4.2.1</h3>\n<ul>\n<li>Bump <code>@actions/cache</code> to v4.0.1</li>\n</ul>\n<h3>4.2.0</h3>\n<p>TLDR; The cache backend service has been rewritten from the ground up\nfor improved performance and reliability. <a\nhref=\"https://github.com/actions/cache\">actions/cache</a> now integrates\nwith the new cache service (v2) APIs.</p>\n<p>The new service will gradually roll out as of <strong>February 1st,\n2025</strong>. The legacy service will also be sunset on the same date.\nChanges in these release are <strong>fully backward\ncompatible</strong>.</p>\n<p><strong>We are deprecating some versions of this action</strong>. We\nrecommend upgrading to version <code>v4</code> or <code>v3</code> as\nsoon as possible before <strong>February 1st, 2025.</strong> (Upgrade\ninstructions below).</p>\n<p>If you are using pinned SHAs, please use the SHAs of versions\n<code>v4.2.0</code> or <code>v3.4.0</code></p>\n<p>If you do not upgrade, all workflow runs using any of the deprecated\n<a href=\"https://github.com/actions/cache\">actions/cache</a> will\nfail.</p>\n<p>Upgrading to the recommended versions will not break your\nworkflows.</p>\n<h3>4.1.2</h3>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/actions/cache/commit/9255dc7a253b0ccc959486e2bca901246202afeb\"><code>9255dc7</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/actions/cache/issues/1686\">#1686</a>\nfrom actions/cache-v5.0.1-release</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/8ff5423e8b66eacab4e638ee52abbd2cb831366a\"><code>8ff5423</code></a>\nchore: release v5.0.1</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/9233019a152bc768059ac1768b8e4403b5da16c1\"><code>9233019</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/actions/cache/issues/1685\">#1685</a>\nfrom salmanmkc/node24-storage-blob-fix</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/b975f2bb844529e1063ad882c609b224bcd66eb6\"><code>b975f2b</code></a>\nfix: add peer property to package-lock.json for dependencies</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/d0a0e1813491d01d574c95f8d189f62622bbb2ae\"><code>d0a0e18</code></a>\nfix: update license files for <code>@​actions/cache</code>,\nfast-xml-parser, and strnum</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/74de208dcfcbe85c0e7154e7b17e4105fe2554ff\"><code>74de208</code></a>\nfix: update <code>@​actions/cache</code> to ^5.0.1 for Node.js 24\npunycode fix</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/ac7f1152ead02e89c14b5456d14ab17591e74cfb\"><code>ac7f115</code></a>\npeer</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/b0f846b50b6061d7a2ca6f1a2fea61d4a65d1a16\"><code>b0f846b</code></a>\nfix: update <code>@​actions/cache</code> with storage-blob fix for\nNode.js 24 punycode depr...</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/a7833574556fa59680c1b7cb190c1735db73ebf0\"><code>a783357</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/actions/cache/issues/1684\">#1684</a>\nfrom actions/prepare-cache-v5-release</li>\n<li><a\nhref=\"https://github.com/actions/cache/commit/3bb0d78750a39cefce0c2b5a0a9801052b4359ad\"><code>3bb0d78</code></a>\ndocs: highlight v5 runner requirement in releases</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/actions/cache/compare/v4...v5\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=actions/cache&package-manager=github_actions&previous-version=4&new-version=5)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2025-12-18T21:36:10-08:00",
          "tree_id": "917db7019d27242b8e3e6f6077f43fe6785363ec",
          "url": "https://github.com/wallstop/fortress-rollback/commit/16edb9ba3f1d0bcf0578b3aa03a59cd0ad34815a"
        },
        "date": 1766122858779,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 106,
            "range": "± 0",
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
            "value": 562,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 796,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1178,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 119888,
            "range": "± 2207",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25449,
            "range": "± 223",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11556,
            "range": "± 155",
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
          "id": "a7aa06db7f5926147a4cc88725eb7afc9f068aea",
          "message": "chore(deps): bump tracing from 0.1.43 to 0.1.44 (#12)\n\nBumps [tracing](https://github.com/tokio-rs/tracing) from 0.1.43 to\n0.1.44.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/tokio-rs/tracing/releases\">tracing's\nreleases</a>.</em></p>\n<blockquote>\n<h2>tracing 0.1.44</h2>\n<h3>Fixed</h3>\n<ul>\n<li>Fix <code>record_all</code> panic (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>)</li>\n</ul>\n<h3>Changed</h3>\n<ul>\n<li><code>tracing-core</code>: updated to 0.1.36 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>)</li>\n</ul>\n<p><a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/pull/3432\">tokio-rs/tracing#3432</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/pull/3440\">tokio-rs/tracing#3440</a></p>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/2d55f6faf9be83e7e4634129fb96813241aac2b8\"><code>2d55f6f</code></a>\nchore: prepare tracing 0.1.44 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3439\">#3439</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/10a9e838a35e6ded79d66af246be2ee05417136d\"><code>10a9e83</code></a>\nchore: prepare tracing-core 0.1.36 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/ee82cf92a8c750f98cfb7a417cc8defb37e26a00\"><code>ee82cf9</code></a>\ntracing: fix record_all panic (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/9978c3663bcd58de14b3cf089ad24cb63d00a922\"><code>9978c36</code></a>\nchore: prepare tracing-mock 0.1.0-beta.3 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3429\">#3429</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/cc44064b3a41cb586bd633f8a024354928e25819\"><code>cc44064</code></a>\nchore: prepare tracing-subscriber 0.3.22 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3428\">#3428</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/tokio-rs/tracing/compare/tracing-0.1.43...tracing-0.1.44\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=tracing&package-manager=cargo&previous-version=0.1.43&new-version=0.1.44)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2025-12-18T21:43:04-08:00",
          "tree_id": "64758fb942516fd0e9f1516fc46795e2b8ae1d7e",
          "url": "https://github.com/wallstop/fortress-rollback/commit/a7aa06db7f5926147a4cc88725eb7afc9f068aea"
        },
        "date": 1766123269708,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 160,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 550,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 801,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1156,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 118992,
            "range": "± 1564",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25473,
            "range": "± 153",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11421,
            "range": "± 194",
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
          "id": "a7aa06db7f5926147a4cc88725eb7afc9f068aea",
          "message": "chore(deps): bump tracing from 0.1.43 to 0.1.44 (#12)\n\nBumps [tracing](https://github.com/tokio-rs/tracing) from 0.1.43 to\n0.1.44.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/tokio-rs/tracing/releases\">tracing's\nreleases</a>.</em></p>\n<blockquote>\n<h2>tracing 0.1.44</h2>\n<h3>Fixed</h3>\n<ul>\n<li>Fix <code>record_all</code> panic (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>)</li>\n</ul>\n<h3>Changed</h3>\n<ul>\n<li><code>tracing-core</code>: updated to 0.1.36 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>)</li>\n</ul>\n<p><a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/pull/3432\">tokio-rs/tracing#3432</a>\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>:\n<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/pull/3440\">tokio-rs/tracing#3440</a></p>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/2d55f6faf9be83e7e4634129fb96813241aac2b8\"><code>2d55f6f</code></a>\nchore: prepare tracing 0.1.44 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3439\">#3439</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/10a9e838a35e6ded79d66af246be2ee05417136d\"><code>10a9e83</code></a>\nchore: prepare tracing-core 0.1.36 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3440\">#3440</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/ee82cf92a8c750f98cfb7a417cc8defb37e26a00\"><code>ee82cf9</code></a>\ntracing: fix record_all panic (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3432\">#3432</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/9978c3663bcd58de14b3cf089ad24cb63d00a922\"><code>9978c36</code></a>\nchore: prepare tracing-mock 0.1.0-beta.3 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3429\">#3429</a>)</li>\n<li><a\nhref=\"https://github.com/tokio-rs/tracing/commit/cc44064b3a41cb586bd633f8a024354928e25819\"><code>cc44064</code></a>\nchore: prepare tracing-subscriber 0.3.22 (<a\nhref=\"https://redirect.github.com/tokio-rs/tracing/issues/3428\">#3428</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/tokio-rs/tracing/compare/tracing-0.1.43...tracing-0.1.44\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=tracing&package-manager=cargo&previous-version=0.1.43&new-version=0.1.44)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2025-12-18T21:43:04-08:00",
          "tree_id": "64758fb942516fd0e9f1516fc46795e2b8ae1d7e",
          "url": "https://github.com/wallstop/fortress-rollback/commit/a7aa06db7f5926147a4cc88725eb7afc9f068aea"
        },
        "date": 1766123273821,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 106,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 159,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 533,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 771,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1157,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 118843,
            "range": "± 1059",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25549,
            "range": "± 1232",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11555,
            "range": "± 102",
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
          "id": "8b42f3b5d94a5652a347e69ccdfa60a42e08fcdd",
          "message": "chore(ci): bump actions/checkout from 4 to 6 + Fix network tests, enhance llm instructions (#10)\n\nBumps [actions/checkout](https://github.com/actions/checkout) from 4 to\n6.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/actions/checkout/releases\">actions/checkout's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v6.0.0</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update README to include Node.js 24 support details and requirements\nby <a href=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2248\">actions/checkout#2248</a></li>\n<li>Persist creds to a separate file by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2286\">actions/checkout#2286</a></li>\n<li>v6-beta by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2298\">actions/checkout#2298</a></li>\n<li>update readme/changelog for v6 by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2311\">actions/checkout#2311</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/checkout/compare/v5.0.0...v6.0.0\">https://github.com/actions/checkout/compare/v5.0.0...v6.0.0</a></p>\n<h2>v6-beta</h2>\n<h2>What's Changed</h2>\n<p>Updated persist-credentials to store the credentials under\n<code>$RUNNER_TEMP</code> instead of directly in the local git\nconfig.</p>\n<p>This requires a minimum Actions Runner version of <a\nhref=\"https://github.com/actions/runner/releases/tag/v2.329.0\">v2.329.0</a>\nto access the persisted credentials for <a\nhref=\"https://docs.github.com/en/actions/tutorials/use-containerized-services/create-a-docker-container-action\">Docker\ncontainer action</a> scenarios.</p>\n<h2>v5.0.1</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Port v6 cleanup to v5 by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2301\">actions/checkout#2301</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/checkout/compare/v5...v5.0.1\">https://github.com/actions/checkout/compare/v5...v5.0.1</a></p>\n<h2>v5.0.0</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update actions checkout to use node 24 by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2226\">actions/checkout#2226</a></li>\n<li>Prepare v5.0.0 release by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2238\">actions/checkout#2238</a></li>\n</ul>\n<h2>⚠️ Minimum Compatible Runner Version</h2>\n<p><strong>v2.327.1</strong><br />\n<a\nhref=\"https://github.com/actions/runner/releases/tag/v2.327.1\">Release\nNotes</a></p>\n<p>Make sure your runner is updated to this version or newer to use this\nrelease.</p>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/checkout/compare/v4...v5.0.0\">https://github.com/actions/checkout/compare/v4...v5.0.0</a></p>\n<h2>v4.3.1</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Port v6 cleanup to v4 by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2305\">actions/checkout#2305</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/checkout/compare/v4...v4.3.1\">https://github.com/actions/checkout/compare/v4...v4.3.1</a></p>\n<h2>v4.3.0</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>docs: update README.md by <a\nhref=\"https://github.com/motss\"><code>@​motss</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1971\">actions/checkout#1971</a></li>\n<li>Add internal repos for checking out multiple repositories by <a\nhref=\"https://github.com/mouismail\"><code>@​mouismail</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1977\">actions/checkout#1977</a></li>\n<li>Documentation update - add recommended permissions to Readme by <a\nhref=\"https://github.com/benwells\"><code>@​benwells</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2043\">actions/checkout#2043</a></li>\n</ul>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/actions/checkout/blob/main/CHANGELOG.md\">actions/checkout's\nchangelog</a>.</em></p>\n<blockquote>\n<h1>Changelog</h1>\n<h2>v6.0.0</h2>\n<ul>\n<li>Persist creds to a separate file by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2286\">actions/checkout#2286</a></li>\n<li>Update README to include Node.js 24 support details and requirements\nby <a href=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2248\">actions/checkout#2248</a></li>\n</ul>\n<h2>v5.0.1</h2>\n<ul>\n<li>Port v6 cleanup to v5 by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2301\">actions/checkout#2301</a></li>\n</ul>\n<h2>v5.0.0</h2>\n<ul>\n<li>Update actions checkout to use node 24 by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2226\">actions/checkout#2226</a></li>\n</ul>\n<h2>v4.3.1</h2>\n<ul>\n<li>Port v6 cleanup to v4 by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2305\">actions/checkout#2305</a></li>\n</ul>\n<h2>v4.3.0</h2>\n<ul>\n<li>docs: update README.md by <a\nhref=\"https://github.com/motss\"><code>@​motss</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1971\">actions/checkout#1971</a></li>\n<li>Add internal repos for checking out multiple repositories by <a\nhref=\"https://github.com/mouismail\"><code>@​mouismail</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1977\">actions/checkout#1977</a></li>\n<li>Documentation update - add recommended permissions to Readme by <a\nhref=\"https://github.com/benwells\"><code>@​benwells</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2043\">actions/checkout#2043</a></li>\n<li>Adjust positioning of user email note and permissions heading by <a\nhref=\"https://github.com/joshmgross\"><code>@​joshmgross</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2044\">actions/checkout#2044</a></li>\n<li>Update README.md by <a\nhref=\"https://github.com/nebuk89\"><code>@​nebuk89</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2194\">actions/checkout#2194</a></li>\n<li>Update CODEOWNERS for actions by <a\nhref=\"https://github.com/TingluoHuang\"><code>@​TingluoHuang</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2224\">actions/checkout#2224</a></li>\n<li>Update package dependencies by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2236\">actions/checkout#2236</a></li>\n</ul>\n<h2>v4.2.2</h2>\n<ul>\n<li><code>url-helper.ts</code> now leverages well-known environment\nvariables by <a href=\"https://github.com/jww3\"><code>@​jww3</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1941\">actions/checkout#1941</a></li>\n<li>Expand unit test coverage for <code>isGhes</code> by <a\nhref=\"https://github.com/jww3\"><code>@​jww3</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1946\">actions/checkout#1946</a></li>\n</ul>\n<h2>v4.2.1</h2>\n<ul>\n<li>Check out other refs/* by commit if provided, fall back to ref by <a\nhref=\"https://github.com/orhantoy\"><code>@​orhantoy</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1924\">actions/checkout#1924</a></li>\n</ul>\n<h2>v4.2.0</h2>\n<ul>\n<li>Add Ref and Commit outputs by <a\nhref=\"https://github.com/lucacome\"><code>@​lucacome</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1180\">actions/checkout#1180</a></li>\n<li>Dependency updates by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a>- <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1777\">actions/checkout#1777</a>,\n<a\nhref=\"https://redirect.github.com/actions/checkout/pull/1872\">actions/checkout#1872</a></li>\n</ul>\n<h2>v4.1.7</h2>\n<ul>\n<li>Bump the minor-npm-dependencies group across 1 directory with 4\nupdates by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1739\">actions/checkout#1739</a></li>\n<li>Bump actions/checkout from 3 to 4 by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1697\">actions/checkout#1697</a></li>\n<li>Check out other refs/* by commit by <a\nhref=\"https://github.com/orhantoy\"><code>@​orhantoy</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1774\">actions/checkout#1774</a></li>\n<li>Pin actions/checkout's own workflows to a known, good, stable\nversion. by <a href=\"https://github.com/jww3\"><code>@​jww3</code></a> in\n<a\nhref=\"https://redirect.github.com/actions/checkout/pull/1776\">actions/checkout#1776</a></li>\n</ul>\n<h2>v4.1.6</h2>\n<ul>\n<li>Check platform to set archive extension appropriately by <a\nhref=\"https://github.com/cory-miller\"><code>@​cory-miller</code></a> in\n<a\nhref=\"https://redirect.github.com/actions/checkout/pull/1732\">actions/checkout#1732</a></li>\n</ul>\n<h2>v4.1.5</h2>\n<ul>\n<li>Update NPM dependencies by <a\nhref=\"https://github.com/cory-miller\"><code>@​cory-miller</code></a> in\n<a\nhref=\"https://redirect.github.com/actions/checkout/pull/1703\">actions/checkout#1703</a></li>\n<li>Bump github/codeql-action from 2 to 3 by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1694\">actions/checkout#1694</a></li>\n<li>Bump actions/setup-node from 1 to 4 by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1696\">actions/checkout#1696</a></li>\n<li>Bump actions/upload-artifact from 2 to 4 by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1695\">actions/checkout#1695</a></li>\n</ul>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/8e8c483db84b4bee98b60c0593521ed34d9990e8\"><code>8e8c483</code></a>\nClarify v6 README (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2328\">#2328</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/033fa0dc0b82693d8986f1016a0ec2c5e7d9cbb1\"><code>033fa0d</code></a>\nAdd worktree support for persist-credentials includeIf (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2327\">#2327</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/c2d88d3ecc89a9ef08eebf45d9637801dcee7eb5\"><code>c2d88d3</code></a>\nUpdate all references from v5 and v4 to v6 (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2314\">#2314</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/1af3b93b6815bc44a9784bd300feb67ff0d1eeb3\"><code>1af3b93</code></a>\nupdate readme/changelog for v6 (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2311\">#2311</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/71cf2267d89c5cb81562390fa70a37fa40b1305e\"><code>71cf226</code></a>\nv6-beta (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2298\">#2298</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/069c6959146423d11cd0184e6accf28f9d45f06e\"><code>069c695</code></a>\nPersist creds to a separate file (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2286\">#2286</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/ff7abcd0c3c05ccf6adc123a8cd1fd4fb30fb493\"><code>ff7abcd</code></a>\nUpdate README to include Node.js 24 support details and requirements (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2248\">#2248</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/08c6903cd8c0fde910a37f88322edcfb5dd907a8\"><code>08c6903</code></a>\nPrepare v5.0.0 release (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2238\">#2238</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/9f265659d3bb64ab1440b03b12f4d47a24320917\"><code>9f26565</code></a>\nUpdate actions checkout to use node 24 (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2226\">#2226</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/actions/checkout/compare/v4...v6\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=actions/checkout&package-manager=github_actions&previous-version=4&new-version=6)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\n---------\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>\nCo-authored-by: wallstop <wallstop@wallstopstudios.com>",
          "timestamp": "2025-12-19T11:40:51-08:00",
          "tree_id": "6782abd60598e7413639832937cfa4ff6d9611ec",
          "url": "https://github.com/wallstop/fortress-rollback/commit/8b42f3b5d94a5652a347e69ccdfa60a42e08fcdd"
        },
        "date": 1766173544785,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 106,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 160,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 552,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 789,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1151,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 119439,
            "range": "± 2094",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25464,
            "range": "± 183",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11378,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
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
          "id": "8b42f3b5d94a5652a347e69ccdfa60a42e08fcdd",
          "message": "chore(ci): bump actions/checkout from 4 to 6 + Fix network tests, enhance llm instructions (#10)\n\nBumps [actions/checkout](https://github.com/actions/checkout) from 4 to\n6.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/actions/checkout/releases\">actions/checkout's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v6.0.0</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update README to include Node.js 24 support details and requirements\nby <a href=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2248\">actions/checkout#2248</a></li>\n<li>Persist creds to a separate file by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2286\">actions/checkout#2286</a></li>\n<li>v6-beta by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2298\">actions/checkout#2298</a></li>\n<li>update readme/changelog for v6 by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2311\">actions/checkout#2311</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/checkout/compare/v5.0.0...v6.0.0\">https://github.com/actions/checkout/compare/v5.0.0...v6.0.0</a></p>\n<h2>v6-beta</h2>\n<h2>What's Changed</h2>\n<p>Updated persist-credentials to store the credentials under\n<code>$RUNNER_TEMP</code> instead of directly in the local git\nconfig.</p>\n<p>This requires a minimum Actions Runner version of <a\nhref=\"https://github.com/actions/runner/releases/tag/v2.329.0\">v2.329.0</a>\nto access the persisted credentials for <a\nhref=\"https://docs.github.com/en/actions/tutorials/use-containerized-services/create-a-docker-container-action\">Docker\ncontainer action</a> scenarios.</p>\n<h2>v5.0.1</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Port v6 cleanup to v5 by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2301\">actions/checkout#2301</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/checkout/compare/v5...v5.0.1\">https://github.com/actions/checkout/compare/v5...v5.0.1</a></p>\n<h2>v5.0.0</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Update actions checkout to use node 24 by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2226\">actions/checkout#2226</a></li>\n<li>Prepare v5.0.0 release by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2238\">actions/checkout#2238</a></li>\n</ul>\n<h2>⚠️ Minimum Compatible Runner Version</h2>\n<p><strong>v2.327.1</strong><br />\n<a\nhref=\"https://github.com/actions/runner/releases/tag/v2.327.1\">Release\nNotes</a></p>\n<p>Make sure your runner is updated to this version or newer to use this\nrelease.</p>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/checkout/compare/v4...v5.0.0\">https://github.com/actions/checkout/compare/v4...v5.0.0</a></p>\n<h2>v4.3.1</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>Port v6 cleanup to v4 by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2305\">actions/checkout#2305</a></li>\n</ul>\n<p><strong>Full Changelog</strong>: <a\nhref=\"https://github.com/actions/checkout/compare/v4...v4.3.1\">https://github.com/actions/checkout/compare/v4...v4.3.1</a></p>\n<h2>v4.3.0</h2>\n<h2>What's Changed</h2>\n<ul>\n<li>docs: update README.md by <a\nhref=\"https://github.com/motss\"><code>@​motss</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1971\">actions/checkout#1971</a></li>\n<li>Add internal repos for checking out multiple repositories by <a\nhref=\"https://github.com/mouismail\"><code>@​mouismail</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1977\">actions/checkout#1977</a></li>\n<li>Documentation update - add recommended permissions to Readme by <a\nhref=\"https://github.com/benwells\"><code>@​benwells</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2043\">actions/checkout#2043</a></li>\n</ul>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Changelog</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/actions/checkout/blob/main/CHANGELOG.md\">actions/checkout's\nchangelog</a>.</em></p>\n<blockquote>\n<h1>Changelog</h1>\n<h2>v6.0.0</h2>\n<ul>\n<li>Persist creds to a separate file by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2286\">actions/checkout#2286</a></li>\n<li>Update README to include Node.js 24 support details and requirements\nby <a href=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2248\">actions/checkout#2248</a></li>\n</ul>\n<h2>v5.0.1</h2>\n<ul>\n<li>Port v6 cleanup to v5 by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2301\">actions/checkout#2301</a></li>\n</ul>\n<h2>v5.0.0</h2>\n<ul>\n<li>Update actions checkout to use node 24 by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2226\">actions/checkout#2226</a></li>\n</ul>\n<h2>v4.3.1</h2>\n<ul>\n<li>Port v6 cleanup to v4 by <a\nhref=\"https://github.com/ericsciple\"><code>@​ericsciple</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2305\">actions/checkout#2305</a></li>\n</ul>\n<h2>v4.3.0</h2>\n<ul>\n<li>docs: update README.md by <a\nhref=\"https://github.com/motss\"><code>@​motss</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1971\">actions/checkout#1971</a></li>\n<li>Add internal repos for checking out multiple repositories by <a\nhref=\"https://github.com/mouismail\"><code>@​mouismail</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1977\">actions/checkout#1977</a></li>\n<li>Documentation update - add recommended permissions to Readme by <a\nhref=\"https://github.com/benwells\"><code>@​benwells</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2043\">actions/checkout#2043</a></li>\n<li>Adjust positioning of user email note and permissions heading by <a\nhref=\"https://github.com/joshmgross\"><code>@​joshmgross</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2044\">actions/checkout#2044</a></li>\n<li>Update README.md by <a\nhref=\"https://github.com/nebuk89\"><code>@​nebuk89</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2194\">actions/checkout#2194</a></li>\n<li>Update CODEOWNERS for actions by <a\nhref=\"https://github.com/TingluoHuang\"><code>@​TingluoHuang</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2224\">actions/checkout#2224</a></li>\n<li>Update package dependencies by <a\nhref=\"https://github.com/salmanmkc\"><code>@​salmanmkc</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/2236\">actions/checkout#2236</a></li>\n</ul>\n<h2>v4.2.2</h2>\n<ul>\n<li><code>url-helper.ts</code> now leverages well-known environment\nvariables by <a href=\"https://github.com/jww3\"><code>@​jww3</code></a>\nin <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1941\">actions/checkout#1941</a></li>\n<li>Expand unit test coverage for <code>isGhes</code> by <a\nhref=\"https://github.com/jww3\"><code>@​jww3</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1946\">actions/checkout#1946</a></li>\n</ul>\n<h2>v4.2.1</h2>\n<ul>\n<li>Check out other refs/* by commit if provided, fall back to ref by <a\nhref=\"https://github.com/orhantoy\"><code>@​orhantoy</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1924\">actions/checkout#1924</a></li>\n</ul>\n<h2>v4.2.0</h2>\n<ul>\n<li>Add Ref and Commit outputs by <a\nhref=\"https://github.com/lucacome\"><code>@​lucacome</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1180\">actions/checkout#1180</a></li>\n<li>Dependency updates by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a>- <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1777\">actions/checkout#1777</a>,\n<a\nhref=\"https://redirect.github.com/actions/checkout/pull/1872\">actions/checkout#1872</a></li>\n</ul>\n<h2>v4.1.7</h2>\n<ul>\n<li>Bump the minor-npm-dependencies group across 1 directory with 4\nupdates by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1739\">actions/checkout#1739</a></li>\n<li>Bump actions/checkout from 3 to 4 by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1697\">actions/checkout#1697</a></li>\n<li>Check out other refs/* by commit by <a\nhref=\"https://github.com/orhantoy\"><code>@​orhantoy</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1774\">actions/checkout#1774</a></li>\n<li>Pin actions/checkout's own workflows to a known, good, stable\nversion. by <a href=\"https://github.com/jww3\"><code>@​jww3</code></a> in\n<a\nhref=\"https://redirect.github.com/actions/checkout/pull/1776\">actions/checkout#1776</a></li>\n</ul>\n<h2>v4.1.6</h2>\n<ul>\n<li>Check platform to set archive extension appropriately by <a\nhref=\"https://github.com/cory-miller\"><code>@​cory-miller</code></a> in\n<a\nhref=\"https://redirect.github.com/actions/checkout/pull/1732\">actions/checkout#1732</a></li>\n</ul>\n<h2>v4.1.5</h2>\n<ul>\n<li>Update NPM dependencies by <a\nhref=\"https://github.com/cory-miller\"><code>@​cory-miller</code></a> in\n<a\nhref=\"https://redirect.github.com/actions/checkout/pull/1703\">actions/checkout#1703</a></li>\n<li>Bump github/codeql-action from 2 to 3 by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1694\">actions/checkout#1694</a></li>\n<li>Bump actions/setup-node from 1 to 4 by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1696\">actions/checkout#1696</a></li>\n<li>Bump actions/upload-artifact from 2 to 4 by <a\nhref=\"https://github.com/dependabot\"><code>@​dependabot</code></a> in <a\nhref=\"https://redirect.github.com/actions/checkout/pull/1695\">actions/checkout#1695</a></li>\n</ul>\n<!-- raw HTML omitted -->\n</blockquote>\n<p>... (truncated)</p>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/8e8c483db84b4bee98b60c0593521ed34d9990e8\"><code>8e8c483</code></a>\nClarify v6 README (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2328\">#2328</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/033fa0dc0b82693d8986f1016a0ec2c5e7d9cbb1\"><code>033fa0d</code></a>\nAdd worktree support for persist-credentials includeIf (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2327\">#2327</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/c2d88d3ecc89a9ef08eebf45d9637801dcee7eb5\"><code>c2d88d3</code></a>\nUpdate all references from v5 and v4 to v6 (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2314\">#2314</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/1af3b93b6815bc44a9784bd300feb67ff0d1eeb3\"><code>1af3b93</code></a>\nupdate readme/changelog for v6 (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2311\">#2311</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/71cf2267d89c5cb81562390fa70a37fa40b1305e\"><code>71cf226</code></a>\nv6-beta (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2298\">#2298</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/069c6959146423d11cd0184e6accf28f9d45f06e\"><code>069c695</code></a>\nPersist creds to a separate file (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2286\">#2286</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/ff7abcd0c3c05ccf6adc123a8cd1fd4fb30fb493\"><code>ff7abcd</code></a>\nUpdate README to include Node.js 24 support details and requirements (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2248\">#2248</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/08c6903cd8c0fde910a37f88322edcfb5dd907a8\"><code>08c6903</code></a>\nPrepare v5.0.0 release (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2238\">#2238</a>)</li>\n<li><a\nhref=\"https://github.com/actions/checkout/commit/9f265659d3bb64ab1440b03b12f4d47a24320917\"><code>9f26565</code></a>\nUpdate actions checkout to use node 24 (<a\nhref=\"https://redirect.github.com/actions/checkout/issues/2226\">#2226</a>)</li>\n<li>See full diff in <a\nhref=\"https://github.com/actions/checkout/compare/v4...v6\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=actions/checkout&package-manager=github_actions&previous-version=4&new-version=6)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\n---------\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>\nCo-authored-by: wallstop <wallstop@wallstopstudios.com>",
          "timestamp": "2025-12-19T11:40:51-08:00",
          "tree_id": "6782abd60598e7413639832937cfa4ff6d9611ec",
          "url": "https://github.com/wallstop/fortress-rollback/commit/8b42f3b5d94a5652a347e69ccdfa60a42e08fcdd"
        },
        "date": 1766173549541,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 160,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 562,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 845,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1191,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 118934,
            "range": "± 3826",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 26073,
            "range": "± 3606",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11564,
            "range": "± 224",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
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
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bbaebbe63dface7dafd2e2ab376abcdbe89b3618",
          "message": "Crate minification (#17)",
          "timestamp": "2025-12-19T15:04:19-08:00",
          "tree_id": "62052b7f6d2a010cc8de20d2ad5e2498b980bf03",
          "url": "https://github.com/wallstop/fortress-rollback/commit/bbaebbe63dface7dafd2e2ab376abcdbe89b3618"
        },
        "date": 1766185749155,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "range": "± 0",
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
            "value": 537,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 757,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1119,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 116297,
            "range": "± 1854",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25437,
            "range": "± 152",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11634,
            "range": "± 154",
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
          "id": "bbaebbe63dface7dafd2e2ab376abcdbe89b3618",
          "message": "Crate minification (#17)",
          "timestamp": "2025-12-19T15:04:19-08:00",
          "tree_id": "62052b7f6d2a010cc8de20d2ad5e2498b980bf03",
          "url": "https://github.com/wallstop/fortress-rollback/commit/bbaebbe63dface7dafd2e2ab376abcdbe89b3618"
        },
        "date": 1766185755874,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 161,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 546,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 819,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1157,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 119536,
            "range": "± 2338",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25480,
            "range": "± 357",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11377,
            "range": "± 155",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
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
          "id": "74001892679241253d1be2289fb271206755e0cb",
          "message": "Further cargo size reduction (#18)",
          "timestamp": "2025-12-19T16:00:35-08:00",
          "tree_id": "008074937a67f8abd8947024eff864af6e889a52",
          "url": "https://github.com/wallstop/fortress-rollback/commit/74001892679241253d1be2289fb271206755e0cb"
        },
        "date": 1766189130242,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 106,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 161,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 532,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 796,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1145,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 121268,
            "range": "± 2181",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25467,
            "range": "± 144",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11657,
            "range": "± 164",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
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
          "id": "74001892679241253d1be2289fb271206755e0cb",
          "message": "Further cargo size reduction (#18)",
          "timestamp": "2025-12-19T16:00:35-08:00",
          "tree_id": "008074937a67f8abd8947024eff864af6e889a52",
          "url": "https://github.com/wallstop/fortress-rollback/commit/74001892679241253d1be2289fb271206755e0cb"
        },
        "date": 1766189133790,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "range": "± 0",
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
            "value": 526,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 802,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1163,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 117230,
            "range": "± 1996",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25368,
            "range": "± 278",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11658,
            "range": "± 452",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
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
          "id": "ec26efb9d49c520b6405fdfeea6569d88d8c6af4",
          "message": "More crate minification, serde_json optional, CI stability (#19)",
          "timestamp": "2025-12-19T23:01:17-08:00",
          "tree_id": "a341c0ac3de5b133007a5a425398dc5375c255f0",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ec26efb9d49c520b6405fdfeea6569d88d8c6af4"
        },
        "date": 1766214370359,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 106,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 161,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 527,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 806,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1122,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 119120,
            "range": "± 1362",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25635,
            "range": "± 163",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11537,
            "range": "± 122",
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
          "id": "ec26efb9d49c520b6405fdfeea6569d88d8c6af4",
          "message": "More crate minification, serde_json optional, CI stability (#19)",
          "timestamp": "2025-12-19T23:01:17-08:00",
          "tree_id": "a341c0ac3de5b133007a5a425398dc5375c255f0",
          "url": "https://github.com/wallstop/fortress-rollback/commit/ec26efb9d49c520b6405fdfeea6569d88d8c6af4"
        },
        "date": 1766214374527,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 106,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 160,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 541,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 781,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1150,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 118561,
            "range": "± 1067",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25206,
            "range": "± 225",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11533,
            "range": "± 148",
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
          "id": "66f784064260303d8e536608db7e893a24419934",
          "message": "Simplify kani proof (#20)",
          "timestamp": "2025-12-19T23:21:51-08:00",
          "tree_id": "56b2a764696704a3adb77c060d7208a67986e441",
          "url": "https://github.com/wallstop/fortress-rollback/commit/66f784064260303d8e536608db7e893a24419934"
        },
        "date": 1766215603873,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 106,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 160,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 507,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 761,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1135,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 119267,
            "range": "± 1010",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25730,
            "range": "± 400",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11546,
            "range": "± 126",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1555,
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
          "id": "66f784064260303d8e536608db7e893a24419934",
          "message": "Simplify kani proof (#20)",
          "timestamp": "2025-12-19T23:21:51-08:00",
          "tree_id": "56b2a764696704a3adb77c060d7208a67986e441",
          "url": "https://github.com/wallstop/fortress-rollback/commit/66f784064260303d8e536608db7e893a24419934"
        },
        "date": 1766215849504,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 106,
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
            "value": 540,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 807,
            "range": "± 13",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1151,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 118786,
            "range": "± 1025",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25404,
            "range": "± 273",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11529,
            "range": "± 121",
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
          "id": "535f085c6a15fafff19639f34aeb38fb25263316",
          "message": "chore(deps): bump serde_json from 1.0.145 to 1.0.146 (#21)\n\nBumps [serde_json](https://github.com/serde-rs/json) from 1.0.145 to\n1.0.146.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/serde-rs/json/releases\">serde_json's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v1.0.146</h2>\n<ul>\n<li>Set fast_arithmetic=64 for riscv64 (<a\nhref=\"https://redirect.github.com/serde-rs/json/issues/1305\">#1305</a>,\nthanks <a\nhref=\"https://github.com/Xeonacid\"><code>@​Xeonacid</code></a>)</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/75ad7e6b4eb8a26560300d2d7332d6dd8cd5b277\"><code>75ad7e6</code></a>\nRelease 1.0.146</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/bc6c8276d9597fae216085f940c712f4d4fce4bc\"><code>bc6c827</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/serde-rs/json/issues/1305\">#1305</a>\nfrom Xeonacid/patch-1</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/a09210adf529842b912db6f69ad9858ad2f90e16\"><code>a09210a</code></a>\nSet fast_arithmetic=64 for riscv64</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/01182e54b5dbadee79696bd472b67391e92679af\"><code>01182e5</code></a>\nUpdate actions/upload-artifact@v5 -&gt; v6</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/383b13a45feb2955236735397c53218acd4da515\"><code>383b13a</code></a>\nUpdate actions/upload-artifact@v4 -&gt; v5</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/04dd357b99699e1abc34c1af2fe52227a74835f5\"><code>04dd357</code></a>\nRaise required compiler to Rust 1.68</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/e047dfbe00b8ba43d1bf78025aabb1a093cea4c0\"><code>e047dfb</code></a>\nResolve manual_let_else pedantic clippy lint</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/a525d9c0c0713bf951319d8bdd25ef102e9241c9\"><code>a525d9c</code></a>\nRaise required compiler to Rust 1.65</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/f815793bfd19cddf311031687dc674578791c49e\"><code>f815793</code></a>\nRemove rustc version badge from readme</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/3f17d2c6ea1588bd7b714359522bd94751465275\"><code>3f17d2c</code></a>\nUpdate actions/checkout@v5 -&gt; v6</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/serde-rs/json/compare/v1.0.145...v1.0.146\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=serde_json&package-manager=cargo&previous-version=1.0.145&new-version=1.0.146)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2025-12-22T20:06:47-08:00",
          "tree_id": "75e753cf9efd0c4cff6ea71f148f774992a72267",
          "url": "https://github.com/wallstop/fortress-rollback/commit/535f085c6a15fafff19639f34aeb38fb25263316"
        },
        "date": 1766463111537,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 106,
            "range": "± 1",
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
            "value": 518,
            "range": "± 7",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 759,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1164,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 118218,
            "range": "± 1037",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25184,
            "range": "± 1147",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11567,
            "range": "± 193",
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
          "id": "535f085c6a15fafff19639f34aeb38fb25263316",
          "message": "chore(deps): bump serde_json from 1.0.145 to 1.0.146 (#21)\n\nBumps [serde_json](https://github.com/serde-rs/json) from 1.0.145 to\n1.0.146.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/serde-rs/json/releases\">serde_json's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v1.0.146</h2>\n<ul>\n<li>Set fast_arithmetic=64 for riscv64 (<a\nhref=\"https://redirect.github.com/serde-rs/json/issues/1305\">#1305</a>,\nthanks <a\nhref=\"https://github.com/Xeonacid\"><code>@​Xeonacid</code></a>)</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/75ad7e6b4eb8a26560300d2d7332d6dd8cd5b277\"><code>75ad7e6</code></a>\nRelease 1.0.146</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/bc6c8276d9597fae216085f940c712f4d4fce4bc\"><code>bc6c827</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/serde-rs/json/issues/1305\">#1305</a>\nfrom Xeonacid/patch-1</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/a09210adf529842b912db6f69ad9858ad2f90e16\"><code>a09210a</code></a>\nSet fast_arithmetic=64 for riscv64</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/01182e54b5dbadee79696bd472b67391e92679af\"><code>01182e5</code></a>\nUpdate actions/upload-artifact@v5 -&gt; v6</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/383b13a45feb2955236735397c53218acd4da515\"><code>383b13a</code></a>\nUpdate actions/upload-artifact@v4 -&gt; v5</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/04dd357b99699e1abc34c1af2fe52227a74835f5\"><code>04dd357</code></a>\nRaise required compiler to Rust 1.68</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/e047dfbe00b8ba43d1bf78025aabb1a093cea4c0\"><code>e047dfb</code></a>\nResolve manual_let_else pedantic clippy lint</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/a525d9c0c0713bf951319d8bdd25ef102e9241c9\"><code>a525d9c</code></a>\nRaise required compiler to Rust 1.65</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/f815793bfd19cddf311031687dc674578791c49e\"><code>f815793</code></a>\nRemove rustc version badge from readme</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/3f17d2c6ea1588bd7b714359522bd94751465275\"><code>3f17d2c</code></a>\nUpdate actions/checkout@v5 -&gt; v6</li>\n<li>Additional commits viewable in <a\nhref=\"https://github.com/serde-rs/json/compare/v1.0.145...v1.0.146\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=serde_json&package-manager=cargo&previous-version=1.0.145&new-version=1.0.146)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2025-12-22T20:06:47-08:00",
          "tree_id": "75e753cf9efd0c4cff6ea71f148f774992a72267",
          "url": "https://github.com/wallstop/fortress-rollback/commit/535f085c6a15fafff19639f34aeb38fb25263316"
        },
        "date": 1766463112137,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 106,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 160,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 530,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 766,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1151,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 120344,
            "range": "± 6178",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25745,
            "range": "± 208",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11431,
            "range": "± 149",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
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
          "id": "b4ea35ba140f0cb15384baa42af11bcd6ed7f19c",
          "message": "chore(deps): bump serde_json from 1.0.146 to 1.0.147 (#22)\n\nBumps [serde_json](https://github.com/serde-rs/json) from 1.0.146 to\n1.0.147.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/serde-rs/json/releases\">serde_json's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v1.0.147</h2>\n<ul>\n<li>Switch float-to-string algorithm from Ryū to Żmij for better f32 and\nf64 serialization performance (<a\nhref=\"https://redirect.github.com/serde-rs/json/issues/1304\">#1304</a>)</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/62d6e8d6158ccc1608fb57d9a8a73cc8d15f5b2a\"><code>62d6e8d</code></a>\nRelease 1.0.147</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/fd829a65beb37d2db296f1a64c22c25ad508d6d8\"><code>fd829a6</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/serde-rs/json/issues/1304\">#1304</a>\nfrom dtolnay/zmij</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/e757a3d8813bfacad8354ae3af89fa19a471da6b\"><code>e757a3d</code></a>\nSwitch from ryu -&gt; zmij for float formatting</li>\n<li>See full diff in <a\nhref=\"https://github.com/serde-rs/json/compare/v1.0.146...v1.0.147\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=serde_json&package-manager=cargo&previous-version=1.0.146&new-version=1.0.147)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2025-12-23T20:11:53-08:00",
          "tree_id": "fe6a77aa931248271d4423d52866ba346a168c50",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b4ea35ba140f0cb15384baa42af11bcd6ed7f19c"
        },
        "date": 1766549800924,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 179,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 522,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 759,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1120,
            "range": "± 35",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 118958,
            "range": "± 2121",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 30860,
            "range": "± 145",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11443,
            "range": "± 155",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
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
          "id": "b4ea35ba140f0cb15384baa42af11bcd6ed7f19c",
          "message": "chore(deps): bump serde_json from 1.0.146 to 1.0.147 (#22)\n\nBumps [serde_json](https://github.com/serde-rs/json) from 1.0.146 to\n1.0.147.\n<details>\n<summary>Release notes</summary>\n<p><em>Sourced from <a\nhref=\"https://github.com/serde-rs/json/releases\">serde_json's\nreleases</a>.</em></p>\n<blockquote>\n<h2>v1.0.147</h2>\n<ul>\n<li>Switch float-to-string algorithm from Ryū to Żmij for better f32 and\nf64 serialization performance (<a\nhref=\"https://redirect.github.com/serde-rs/json/issues/1304\">#1304</a>)</li>\n</ul>\n</blockquote>\n</details>\n<details>\n<summary>Commits</summary>\n<ul>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/62d6e8d6158ccc1608fb57d9a8a73cc8d15f5b2a\"><code>62d6e8d</code></a>\nRelease 1.0.147</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/fd829a65beb37d2db296f1a64c22c25ad508d6d8\"><code>fd829a6</code></a>\nMerge pull request <a\nhref=\"https://redirect.github.com/serde-rs/json/issues/1304\">#1304</a>\nfrom dtolnay/zmij</li>\n<li><a\nhref=\"https://github.com/serde-rs/json/commit/e757a3d8813bfacad8354ae3af89fa19a471da6b\"><code>e757a3d</code></a>\nSwitch from ryu -&gt; zmij for float formatting</li>\n<li>See full diff in <a\nhref=\"https://github.com/serde-rs/json/compare/v1.0.146...v1.0.147\">compare\nview</a></li>\n</ul>\n</details>\n<br />\n\n\n[![Dependabot compatibility\nscore](https://dependabot-badges.githubapp.com/badges/compatibility_score?dependency-name=serde_json&package-manager=cargo&previous-version=1.0.146&new-version=1.0.147)](https://docs.github.com/en/github/managing-security-vulnerabilities/about-dependabot-security-updates#about-compatibility-scores)\n\nDependabot will resolve any conflicts with this PR as long as you don't\nalter it yourself. You can also trigger a rebase manually by commenting\n`@dependabot rebase`.\n\n[//]: # (dependabot-automerge-start)\n[//]: # (dependabot-automerge-end)\n\n---\n\n<details>\n<summary>Dependabot commands and options</summary>\n<br />\n\nYou can trigger Dependabot actions by commenting on this PR:\n- `@dependabot rebase` will rebase this PR\n- `@dependabot recreate` will recreate this PR, overwriting any edits\nthat have been made to it\n- `@dependabot merge` will merge this PR after your CI passes on it\n- `@dependabot squash and merge` will squash and merge this PR after\nyour CI passes on it\n- `@dependabot cancel merge` will cancel a previously requested merge\nand block automerging\n- `@dependabot reopen` will reopen this PR if it is closed\n- `@dependabot close` will close this PR and stop Dependabot recreating\nit. You can achieve the same result by closing it manually\n- `@dependabot show <dependency name> ignore conditions` will show all\nof the ignore conditions of the specified dependency\n- `@dependabot ignore this major version` will close this PR and stop\nDependabot creating any more for this major version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this minor version` will close this PR and stop\nDependabot creating any more for this minor version (unless you reopen\nthe PR or upgrade to it yourself)\n- `@dependabot ignore this dependency` will close this PR and stop\nDependabot creating any more for this dependency (unless you reopen the\nPR or upgrade to it yourself)\n\n\n</details>\n\nSigned-off-by: dependabot[bot] <support@github.com>\nCo-authored-by: dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
          "timestamp": "2025-12-23T20:11:53-08:00",
          "tree_id": "fe6a77aa931248271d4423d52866ba346a168c50",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b4ea35ba140f0cb15384baa42af11bcd6ed7f19c"
        },
        "date": 1766549806259,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 178,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 530,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 787,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1163,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 122345,
            "range": "± 7173",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 29930,
            "range": "± 982",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11657,
            "range": "± 259",
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
          "id": "b7d9b727424239b31e763ecdf7e2fc813b2d894b",
          "message": "Investigate and fix test failures (#23)\n\nThe ChaosSocket reorder buffer had a bug where packets would be held\nindefinitely if fewer packets arrived than the configured buffer\nthreshold. This caused the flaky test_out_of_order_delivery test failure\non macOS.\n\nRoot cause analysis:\n- The apply_reordering() function only released packets when the buffer\nreached the configured threshold size (e.g., 5 packets)\n- If fewer packets arrived, they remained stuck in the buffer forever\n- This caused P2P sessions to stall because remote inputs never arrived\n\nFix:\n- Release buffered packets when no new messages arrive (empty batch)\n- This simulates real-world behavior where delayed packets eventually\narrive\n- Added should_reorder() helper for semantic clarity (replaces\nshould_drop)\n\nAlso added comprehensive unit tests for reorder buffer edge cases:\n- test_reorder_buffer_releases_on_empty_batch\n- test_reorder_buffer_releases_at_threshold\n- test_reorder_buffer_no_indefinite_holding\n- test_reorder_buffer_single_packet\n- test_reorder_buffer_disabled_when_size_zero\n- test_reorder_buffer_disabled_when_rate_zero\n- test_reorder_buffer_burst_exceeds_size\n- test_reorder_stats_accuracy\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-23T21:19:49-08:00",
          "tree_id": "fa036c7223d335e1552f6292bd69872746ebc016",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b7d9b727424239b31e763ecdf7e2fc813b2d894b"
        },
        "date": 1766553880040,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 179,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 544,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 751,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1126,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 118978,
            "range": "± 1995",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 30837,
            "range": "± 402",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11662,
            "range": "± 162",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
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
          "id": "b7d9b727424239b31e763ecdf7e2fc813b2d894b",
          "message": "Investigate and fix test failures (#23)\n\nThe ChaosSocket reorder buffer had a bug where packets would be held\nindefinitely if fewer packets arrived than the configured buffer\nthreshold. This caused the flaky test_out_of_order_delivery test failure\non macOS.\n\nRoot cause analysis:\n- The apply_reordering() function only released packets when the buffer\nreached the configured threshold size (e.g., 5 packets)\n- If fewer packets arrived, they remained stuck in the buffer forever\n- This caused P2P sessions to stall because remote inputs never arrived\n\nFix:\n- Release buffered packets when no new messages arrive (empty batch)\n- This simulates real-world behavior where delayed packets eventually\narrive\n- Added should_reorder() helper for semantic clarity (replaces\nshould_drop)\n\nAlso added comprehensive unit tests for reorder buffer edge cases:\n- test_reorder_buffer_releases_on_empty_batch\n- test_reorder_buffer_releases_at_threshold\n- test_reorder_buffer_no_indefinite_holding\n- test_reorder_buffer_single_packet\n- test_reorder_buffer_disabled_when_size_zero\n- test_reorder_buffer_disabled_when_rate_zero\n- test_reorder_buffer_burst_exceeds_size\n- test_reorder_stats_accuracy\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-23T21:19:49-08:00",
          "tree_id": "fa036c7223d335e1552f6292bd69872746ebc016",
          "url": "https://github.com/wallstop/fortress-rollback/commit/b7d9b727424239b31e763ecdf7e2fc813b2d894b"
        },
        "date": 1766553880334,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 153,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 676,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 976,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1426,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 115560,
            "range": "± 3381",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24522,
            "range": "± 122",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 10673,
            "range": "± 484",
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
          "id": "5228d9573c06f8d0c8305b301f484340e7065513",
          "message": "Apply static analysis checks, harder clippy warnings (#24)\n\nAdd ci-safety.yml workflow with:\n- Cargo careful: Runtime safety checks using nightly\n- Strict builds: overflow-checks and debug-assertions in release\n- Panic pattern analysis: Track unwrap/expect/panic/todo usage\n- Strict clippy: Nursery lints enabled\n- Documentation verification: Warnings as errors\n\nEnhance Cargo.toml with:\n- No-panic lints: unwrap_used, expect_used, panic (warn level)\n- Nursery clippy lints enabled as baseline\n- Release profile with overflow-checks enabled\n- New release-safe profile for maximum runtime checking\n\nUpdate .llm/context.md with safety check documentation.\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-25T13:20:15-08:00",
          "tree_id": "cdb3cb2c874cf27027277658175b335f9c7669c7",
          "url": "https://github.com/wallstop/fortress-rollback/commit/5228d9573c06f8d0c8305b301f484340e7065513"
        },
        "date": 1766697924913,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 195,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 555,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 833,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1204,
            "range": "± 36",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 112080,
            "range": "± 1511",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24728,
            "range": "± 585",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11490,
            "range": "± 159",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
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
          "id": "5228d9573c06f8d0c8305b301f484340e7065513",
          "message": "Apply static analysis checks, harder clippy warnings (#24)\n\nAdd ci-safety.yml workflow with:\n- Cargo careful: Runtime safety checks using nightly\n- Strict builds: overflow-checks and debug-assertions in release\n- Panic pattern analysis: Track unwrap/expect/panic/todo usage\n- Strict clippy: Nursery lints enabled\n- Documentation verification: Warnings as errors\n\nEnhance Cargo.toml with:\n- No-panic lints: unwrap_used, expect_used, panic (warn level)\n- Nursery clippy lints enabled as baseline\n- Release profile with overflow-checks enabled\n- New release-safe profile for maximum runtime checking\n\nUpdate .llm/context.md with safety check documentation.\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-25T13:20:15-08:00",
          "tree_id": "cdb3cb2c874cf27027277658175b335f9c7669c7",
          "url": "https://github.com/wallstop/fortress-rollback/commit/5228d9573c06f8d0c8305b301f484340e7065513"
        },
        "date": 1766697927427,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 196,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 540,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 812,
            "range": "± 51",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1177,
            "range": "± 33",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 112002,
            "range": "± 1496",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24779,
            "range": "± 209",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11251,
            "range": "± 162",
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
          "id": "e921aa3e3fc158b440bc913ce5f418f00d7e3a13",
          "message": "Enforce stricter code quality and static analysis (#25)\n\nAdd stronger clippy lints and new CI quality checks:\n\n## Cargo.toml changes\n- Enable clippy::cargo lint group for Cargo.toml hygiene\n- Add cherry-picked restriction lints for safety:\n  - mem_forget (deny): prevent memory leaks\n  - exit (deny): library should never call std::process::exit\n  - string_slice (warn): catch potential panics on string boundaries\n  - map_err_ignore (warn): preserve error context\n  - rest_pat_in_fully_bound_structs (warn): catch new struct fields\n  - verbose_file_reads (warn): prefer fs::read_to_string()\n  - rc_buffer, rc_mutex (warn): smart pointer hygiene\n- Fix lint name: unchecked_time_subtraction ->\nunchecked_duration_subtraction\n\n## clippy.toml changes\n- Add allow-unwrap-in-tests, allow-expect-in-tests,\nallow-indexing-slicing-in-tests for cleaner test code\n\n## CI changes\n- Add ci-quality.yml workflow with:\n  - cargo-shear for fast unused dependency detection\n  - cargo-spellcheck for documentation spelling checks\n- These are advisory checks that don't block builds\n\n## Code fixes\n- Fix Arc::clone style in game_state_cell.rs (clone_on_ref_ptr lint)\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-25T14:15:55-08:00",
          "tree_id": "8c64aa6398f62472942a2d1652abe6f30aefc85c",
          "url": "https://github.com/wallstop/fortress-rollback/commit/e921aa3e3fc158b440bc913ce5f418f00d7e3a13"
        },
        "date": 1766701250388,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 193,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 544,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 789,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1190,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 111373,
            "range": "± 1455",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 25281,
            "range": "± 263",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11231,
            "range": "± 167",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
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
          "id": "e921aa3e3fc158b440bc913ce5f418f00d7e3a13",
          "message": "Enforce stricter code quality and static analysis (#25)\n\nAdd stronger clippy lints and new CI quality checks:\n\n## Cargo.toml changes\n- Enable clippy::cargo lint group for Cargo.toml hygiene\n- Add cherry-picked restriction lints for safety:\n  - mem_forget (deny): prevent memory leaks\n  - exit (deny): library should never call std::process::exit\n  - string_slice (warn): catch potential panics on string boundaries\n  - map_err_ignore (warn): preserve error context\n  - rest_pat_in_fully_bound_structs (warn): catch new struct fields\n  - verbose_file_reads (warn): prefer fs::read_to_string()\n  - rc_buffer, rc_mutex (warn): smart pointer hygiene\n- Fix lint name: unchecked_time_subtraction ->\nunchecked_duration_subtraction\n\n## clippy.toml changes\n- Add allow-unwrap-in-tests, allow-expect-in-tests,\nallow-indexing-slicing-in-tests for cleaner test code\n\n## CI changes\n- Add ci-quality.yml workflow with:\n  - cargo-shear for fast unused dependency detection\n  - cargo-spellcheck for documentation spelling checks\n- These are advisory checks that don't block builds\n\n## Code fixes\n- Fix Arc::clone style in game_state_cell.rs (clone_on_ref_ptr lint)\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-25T14:15:55-08:00",
          "tree_id": "8c64aa6398f62472942a2d1652abe6f30aefc85c",
          "url": "https://github.com/wallstop/fortress-rollback/commit/e921aa3e3fc158b440bc913ce5f418f00d7e3a13"
        },
        "date": 1766701260302,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 194,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 545,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 810,
            "range": "± 22",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1187,
            "range": "± 31",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 112001,
            "range": "± 1871",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24716,
            "range": "± 228",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 11491,
            "range": "± 162",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
            "range": "± 81",
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
          "id": "1b991704d0ab776a816f69629a6ca4cf90cf7f74",
          "message": "Enhance code quality with stricter linting (#26)\n\n- Enable LTO (thin) and codegen-units=1 in release profile for 10-20%\nperf gain\n- Add #![deny(warnings)] to lib.rs for local warnings=errors parity with\nCI\n- Add high-value restriction clippy lints:\n  - if_then_some_else_none: Prefer .then()/.then_some() patterns\n  - empty_drop: Catch empty Drop implementations\n  - create_dir: Prefer create_dir_all() for robustness\n  - mutex_atomic: Prefer AtomicBool over Mutex<bool>\n  - string_add: Prefer push_str() for efficiency\n  - unnecessary_safety_comment/doc: Remove spurious safety docs\n- Refactor code to use idiomatic .then_some() patterns in:\n  - sync_layer/mod.rs: saved_cell_for_frame()\n  - sessions/player_registry.rs: handles_by_address()\n  - tests/network/multi_process.rs: find_peer_binary()\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-25T14:49:47-08:00",
          "tree_id": "662e2f93151564a4421a186c0c5c215559c4b503",
          "url": "https://github.com/wallstop/fortress-rollback/commit/1b991704d0ab776a816f69629a6ca4cf90cf7f74"
        },
        "date": 1766703304526,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 94,
            "range": "± 4",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 138,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 506,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 746,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1045,
            "range": "± 28",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102332,
            "range": "± 549",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27633,
            "range": "± 810",
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
            "value": 1553,
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
          "id": "1b991704d0ab776a816f69629a6ca4cf90cf7f74",
          "message": "Enhance code quality with stricter linting (#26)\n\n- Enable LTO (thin) and codegen-units=1 in release profile for 10-20%\nperf gain\n- Add #![deny(warnings)] to lib.rs for local warnings=errors parity with\nCI\n- Add high-value restriction clippy lints:\n  - if_then_some_else_none: Prefer .then()/.then_some() patterns\n  - empty_drop: Catch empty Drop implementations\n  - create_dir: Prefer create_dir_all() for robustness\n  - mutex_atomic: Prefer AtomicBool over Mutex<bool>\n  - string_add: Prefer push_str() for efficiency\n  - unnecessary_safety_comment/doc: Remove spurious safety docs\n- Refactor code to use idiomatic .then_some() patterns in:\n  - sync_layer/mod.rs: saved_cell_for_frame()\n  - sessions/player_registry.rs: handles_by_address()\n  - tests/network/multi_process.rs: find_peer_binary()\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-25T14:49:47-08:00",
          "tree_id": "662e2f93151564a4421a186c0c5c215559c4b503",
          "url": "https://github.com/wallstop/fortress-rollback/commit/1b991704d0ab776a816f69629a6ca4cf90cf7f74"
        },
        "date": 1766703310037,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 83,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 124,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 624,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 896,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1311,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 95678,
            "range": "± 449",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24349,
            "range": "± 323",
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
            "email": "wallstop@wallstopstudios.com",
            "name": "Eli Pinkerton",
            "username": "wallstop"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d1404255461374b9390a4fe3c7bc56079fbf0bbc",
          "message": "Even stricter clippy lints (#27)\n\n## Static Analysis Improvements\n- Add 15+ additional restriction lints with documented rationale:\n  - format_push_string, mixed_read_write_in_expression\n  - as_underscore, default_union_representation\n  - empty_structs_with_brackets, error_impl_error\n  - fn_to_numeric_cast_any, infinite_loop\n  - iter_over_hash_type, needless_raw_strings\n  - renamed_function_params, try_err\n  - undocumented_unsafe_blocks\n- Document explicitly why certain strict lints are NOT enabled:\n  - std_instead_of_core (not no_std compatible)\n  - clone_on_ref_ptr (arc.clone() is idiomatic)\n  - min_ident_chars (triggers on idiomatic patterns)\n  - impl_trait_in_params (impl AsRef is clearer)\n  - And 8 more with detailed rationale\n\n## Panic Safety Improvements\n- Add checked/saturating arithmetic methods to Frame type:\n  - checked_add, checked_sub: return None on overflow\n  - saturating_add, saturating_sub: clamp at bounds\n  - abs_diff: safe frame distance calculation\n- All new methods are const, inline, and documented with examples\n\n## Performance Optimizations\n- Add #[inline] hints to small hot-path functions in frame_info.rs\n- Add Vec::with_capacity hints where sizes are known:\n  - input_bytes.rs: from_inputs and to_player_inputs\n  - rle.rs: noncontiguous_bits pre-allocation\n- Add #[must_use] to PlayerInput::new\n\nAll tests pass (888 tests).\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-25T17:01:36-08:00",
          "tree_id": "c8c02e8b78386c62ea91fe817d6ec3d1af901dfc",
          "url": "https://github.com/wallstop/fortress-rollback/commit/d1404255461374b9390a4fe3c7bc56079fbf0bbc"
        },
        "date": 1766711198374,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 152,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 491,
            "range": "± 14",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 738,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1069,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102590,
            "range": "± 506",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27549,
            "range": "± 828",
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
          "id": "d1404255461374b9390a4fe3c7bc56079fbf0bbc",
          "message": "Even stricter clippy lints (#27)\n\n## Static Analysis Improvements\n- Add 15+ additional restriction lints with documented rationale:\n  - format_push_string, mixed_read_write_in_expression\n  - as_underscore, default_union_representation\n  - empty_structs_with_brackets, error_impl_error\n  - fn_to_numeric_cast_any, infinite_loop\n  - iter_over_hash_type, needless_raw_strings\n  - renamed_function_params, try_err\n  - undocumented_unsafe_blocks\n- Document explicitly why certain strict lints are NOT enabled:\n  - std_instead_of_core (not no_std compatible)\n  - clone_on_ref_ptr (arc.clone() is idiomatic)\n  - min_ident_chars (triggers on idiomatic patterns)\n  - impl_trait_in_params (impl AsRef is clearer)\n  - And 8 more with detailed rationale\n\n## Panic Safety Improvements\n- Add checked/saturating arithmetic methods to Frame type:\n  - checked_add, checked_sub: return None on overflow\n  - saturating_add, saturating_sub: clamp at bounds\n  - abs_diff: safe frame distance calculation\n- All new methods are const, inline, and documented with examples\n\n## Performance Optimizations\n- Add #[inline] hints to small hot-path functions in frame_info.rs\n- Add Vec::with_capacity hints where sizes are known:\n  - input_bytes.rs: from_inputs and to_player_inputs\n  - rle.rs: noncontiguous_bits pre-allocation\n- Add #[must_use] to PlayerInput::new\n\nAll tests pass (888 tests).\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-25T17:01:36-08:00",
          "tree_id": "c8c02e8b78386c62ea91fe817d6ec3d1af901dfc",
          "url": "https://github.com/wallstop/fortress-rollback/commit/d1404255461374b9390a4fe3c7bc56079fbf0bbc"
        },
        "date": 1766711200491,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 152,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 498,
            "range": "± 42",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 750,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1024,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102487,
            "range": "± 653",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27678,
            "range": "± 844",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1556,
            "range": "± 81",
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
          "id": "8b78adcba52af1d1e7bba5d1049dd82117a2fa3c",
          "message": "Minor optimizations (#30)\n\nThis commit implements 7 improvements identified in a deep project\nanalysis across correctness, performance, usability, and\nmaintainability:\n\n1. Return Result from builder config methods (Ease-of-Use)\n- with_input_delay() and with_num_players() now return Result<Self,\nFortressError>\n   - Prevents silent configuration bugs from clamping invalid values\n   - Updated all call sites in examples, tests, and benchmarks\n\n2. Replace floating-point with integer arithmetic in TimeSync\n(Correctness)\n   - average_frame_advantage() now uses integer-only calculation\n   - Eliminates potential non-determinism from floating-point operations\n\n3. Reduce cloning in poll_remote_clients() (Performance)\n- Changed handles storage from Vec<PlayerHandle> to Arc<[PlayerHandle]>\n   - Cloning is now O(1) atomic increment instead of O(n) vector copy\n   - Critical for 60+ FPS game loops\n\n4. Pre-allocate compression buffers (Performance)\n- delta_decode() uses Vec::with_capacity() to avoid zero-initialization\n   - RLE encoding uses stack-allocated buffers for varint encoding\n   - Reduces allocations in network hot paths\n\n5. Add rollback algorithm overview documentation (Understanding)\n   - Added comprehensive module-level docs to sync_layer/mod.rs\n   - Explains state saving, prediction, rollback, and re-simulation\n   - Includes data flow diagram and determinism requirements\n\n6. Use descriptive variable names (Understanding)\n- Renamed single-letter variables in rng.rs, chaos_socket.rs,\ncompression.rs\n   - e.g., 'r' → 'random_value', 'b1/b2' → 'reference_byte/input_byte'\n\n7. Simplify vector initialization patterns (Maintainability)\n   - Replaced imperative Vec::new() + loop patterns with vec![T; n]\n- Applied across p2p_session.rs, spectator_session.rs,\nsync_test_session.rs\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-25T20:56:01-08:00",
          "tree_id": "453524834d826186a61731bafbceaba57a3d80db",
          "url": "https://github.com/wallstop/fortress-rollback/commit/8b78adcba52af1d1e7bba5d1049dd82117a2fa3c"
        },
        "date": 1766725244013,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 152,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 511,
            "range": "± 16",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 739,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1052,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 103367,
            "range": "± 3054",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27426,
            "range": "± 820",
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
            "range": "± 97",
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
          "id": "8b78adcba52af1d1e7bba5d1049dd82117a2fa3c",
          "message": "Minor optimizations (#30)\n\nThis commit implements 7 improvements identified in a deep project\nanalysis across correctness, performance, usability, and\nmaintainability:\n\n1. Return Result from builder config methods (Ease-of-Use)\n- with_input_delay() and with_num_players() now return Result<Self,\nFortressError>\n   - Prevents silent configuration bugs from clamping invalid values\n   - Updated all call sites in examples, tests, and benchmarks\n\n2. Replace floating-point with integer arithmetic in TimeSync\n(Correctness)\n   - average_frame_advantage() now uses integer-only calculation\n   - Eliminates potential non-determinism from floating-point operations\n\n3. Reduce cloning in poll_remote_clients() (Performance)\n- Changed handles storage from Vec<PlayerHandle> to Arc<[PlayerHandle]>\n   - Cloning is now O(1) atomic increment instead of O(n) vector copy\n   - Critical for 60+ FPS game loops\n\n4. Pre-allocate compression buffers (Performance)\n- delta_decode() uses Vec::with_capacity() to avoid zero-initialization\n   - RLE encoding uses stack-allocated buffers for varint encoding\n   - Reduces allocations in network hot paths\n\n5. Add rollback algorithm overview documentation (Understanding)\n   - Added comprehensive module-level docs to sync_layer/mod.rs\n   - Explains state saving, prediction, rollback, and re-simulation\n   - Includes data flow diagram and determinism requirements\n\n6. Use descriptive variable names (Understanding)\n- Renamed single-letter variables in rng.rs, chaos_socket.rs,\ncompression.rs\n   - e.g., 'r' → 'random_value', 'b1/b2' → 'reference_byte/input_byte'\n\n7. Simplify vector initialization patterns (Maintainability)\n   - Replaced imperative Vec::new() + loop patterns with vec![T; n]\n- Applied across p2p_session.rs, spectator_session.rs,\nsync_test_session.rs\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-25T20:56:01-08:00",
          "tree_id": "453524834d826186a61731bafbceaba57a3d80db",
          "url": "https://github.com/wallstop/fortress-rollback/commit/8b78adcba52af1d1e7bba5d1049dd82117a2fa3c"
        },
        "date": 1766725248592,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 152,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 535,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 743,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1067,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 102731,
            "range": "± 409",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27256,
            "range": "± 1073",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 15",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1554,
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
          "id": "e7b55e500c7ff278aa573642e50d68e6cae11f5f",
          "message": "More configuration options + linter errors + bugfix for sync timeout event flooding (#31)\n\n- Make event queue size configurable via\nSessionBuilder::with_event_queue_size()\n  - Added event_queue_size field to SessionBuilder\n  - Pass through to P2PSession and SpectatorSession\n  - Validates minimum size of 10 events\n\n- Move timeout constant to ProtocolConfig\n- Added input_history_multiplier field with presets (competitive=2,\nhigh_latency=3)\n- Added validate() method to ProtocolConfig for configuration validation\n\n- Add #[must_use] attributes to key session methods\n  - advance_frame() on all session types\n  - disconnect_player() on P2PSession\n  - confirmed_inputs_for_frame() on P2PSession\n  - network_stats() on SpectatorSession\n\n- Add explicit Error::source() implementation with documentation\n  - Explains design choice of using strings vs wrapped errors\n\n- Consolidate test utilities in tests/common/test_utils.rs\n  - Shared constants (MAX_SYNC_ITERATIONS, POLL_INTERVAL, SYNC_TIMEOUT)\n  - Helper functions for session synchronization\n  - ChaosSocket test helper\n\n- Add mutation testing infrastructure\n  - .cargo/mutants.toml configuration\n  - .github/workflows/ci-mutation.yml for monthly scheduled runs\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-26T11:42:56-08:00",
          "tree_id": "03a5d823325056b7d83fe787831424f1fb7c3b36",
          "url": "https://github.com/wallstop/fortress-rollback/commit/e7b55e500c7ff278aa573642e50d68e6cae11f5f"
        },
        "date": 1766778453013,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_no_rollback/4",
            "value": 136,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 640,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 963,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1387,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 97757,
            "range": "± 1045",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 24653,
            "range": "± 676",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 676,
            "range": "± 8",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 868,
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
          "id": "e7b55e500c7ff278aa573642e50d68e6cae11f5f",
          "message": "More configuration options + linter errors + bugfix for sync timeout event flooding (#31)\n\n- Make event queue size configurable via\nSessionBuilder::with_event_queue_size()\n  - Added event_queue_size field to SessionBuilder\n  - Pass through to P2PSession and SpectatorSession\n  - Validates minimum size of 10 events\n\n- Move timeout constant to ProtocolConfig\n- Added input_history_multiplier field with presets (competitive=2,\nhigh_latency=3)\n- Added validate() method to ProtocolConfig for configuration validation\n\n- Add #[must_use] attributes to key session methods\n  - advance_frame() on all session types\n  - disconnect_player() on P2PSession\n  - confirmed_inputs_for_frame() on P2PSession\n  - network_stats() on SpectatorSession\n\n- Add explicit Error::source() implementation with documentation\n  - Explains design choice of using strings vs wrapped errors\n\n- Consolidate test utilities in tests/common/test_utils.rs\n  - Shared constants (MAX_SYNC_ITERATIONS, POLL_INTERVAL, SYNC_TIMEOUT)\n  - Helper functions for session synchronization\n  - ChaosSocket test helper\n\n- Add mutation testing infrastructure\n  - .cargo/mutants.toml configuration\n  - .github/workflows/ci-mutation.yml for monthly scheduled runs\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-26T11:42:56-08:00",
          "tree_id": "03a5d823325056b7d83fe787831424f1fb7c3b36",
          "url": "https://github.com/wallstop/fortress-rollback/commit/e7b55e500c7ff278aa573642e50d68e6cae11f5f"
        },
        "date": 1766778477129,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 151,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 500,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 756,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1069,
            "range": "± 34",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 104197,
            "range": "± 976",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27372,
            "range": "± 817",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_deserialize",
            "value": 1242,
            "range": "± 5",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_encode_into_buffer",
            "value": 1553,
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
          "id": "0c8e43684bbf9019783dd3325da5fe4ef2f8b313",
          "message": "Eliminate non-determinism for cross-platform consistency (#32)\n\nThis change addresses non-determinism in the network protocol by:\n\n1. Replacing DefaultHasher with DeterministicHasher in\ntiming_entropy_seed()\n   - DefaultHasher uses a random seed which varies between runs\n- DeterministicHasher (FNV-1a) provides consistent hashing across\nplatforms\n\n2. Adding optional protocol_rng_seed to ProtocolConfig\n   - When set, provides fully deterministic protocol behavior\n   - Useful for replay systems, deterministic testing, and debugging\n   - New ProtocolConfig::deterministic(seed) preset for convenience\n\n3. Updating UdpProtocol to use seeded RNG for:\n   - Magic number generation (session identifiers)\n   - Sync request validation tokens\n\nThis enables fully reproducible network sessions when configured,\nessential for cross-platform determinism in rollback networking.\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-12-26T12:25:08-08:00",
          "tree_id": "ebd87eaa42fc0bc9ae6972a7b3ae33efe6f8aa56",
          "url": "https://github.com/wallstop/fortress-rollback/commit/0c8e43684bbf9019783dd3325da5fe4ef2f8b313"
        },
        "date": 1766781001084,
        "tool": "cargo",
        "benches": [
          {
            "name": "Frame/new",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_null",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame/is_valid",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "Frame arithmetic/add/100",
            "value": 0,
            "range": "± 0",
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
            "value": 151,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/2",
            "value": 490,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/4",
            "value": 730,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "SyncTestSession/advance_frame_with_rollback/7",
            "value": 1043,
            "range": "± 29",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/round_trip_input_msg",
            "value": 104087,
            "range": "± 406",
            "unit": "ns/iter"
          },
          {
            "name": "Message serialization/input_serialize",
            "value": 27547,
            "range": "± 951",
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
      }
    ]
  }
}