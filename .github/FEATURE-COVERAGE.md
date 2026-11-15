# Feature Coverage Matrix

Every supported feature has a fast lane and a deep lane. A feature is not
considered covered if it only appears in documentation or a compile check.

| Supported surface | PR fast lane | Merge-queue lane | Nightly / weekly lane |
| --- | --- | --- | --- |
| P2P rollback session | Linux nextest suite | Cross-OS build and test | N=16 simulation fleet |
| SyncTest deterministic loop | Linux nextest suite | Cross-OS build and test | Full simulation census |
| Spectator sessions | Linux nextest suite | Cross-OS build and test | N=16 spectator convergence |
| Graceful peer drop | Linux nextest suite | Network tests | Multi-process chaos and soak |
| Hot-join | Linux nextest suite | Hot Join feature matrix | Hot-join lifecycle and failover |
| Input prediction and delay | Linux nextest suite | Network tests | Pacing and liveness cliffs |
| Tokio transport | Clippy and Linux tests | Cross-OS build and test | Network nightly |
| JSON telemetry | Clippy and Linux tests | Cross-OS build and test | Session metrics probes |
| WASM `unknown` target | WASM compile checks | Cross-OS build and test | Browser runtime smoke |
| Emscripten target | WASM compile checks | Cross-OS build and test | Godot browser exports |
| Godot integration | Compile-only checks | Cross-OS build and test | Weekly Godot browser export |
| Deterministic simulation | Linux nextest suite | Simulation smoke | Nightly sharded fleet |
| Replay and checksum corpora | Linux nextest suite | Cross-OS build and test | Extended replay corpus |
| Wire and parser safety | Linux nextest suite | Cross-OS build and test | Fuzz corpus regression |
| Kani proofs | None | Kani Tier 1 | Kani Tiers 2 and 3 |
| TLA+ models | None | TLA+ on `specs/**` changes | Full weekly TLA+ sweep |
| Z3 proofs | None | Z3 verification | Weekly Z3 sweep |
| Loom concurrency | None | Loom on Linux | Loom on all runner OSes |
| Miri undefined behavior | None | Miri on Linux | Full Miri OS matrix |
| Mutation quality | None | Changed-line mutation | Monthly full mutation sweep |
| Coverage | None | Tarpaulin report | Nightly coverage trend |
| Benchmark regressions | None | Paired benchmark gate | Benchmark history |
| Documentation | Docs checks when docs change | Link, prose, and example gates | Wiki and docs deploy |

## Invariants

- The `CI Gate` job is the only required aggregate PR/merge-queue check.
- Deep lanes run on `merge_group`, `schedule`, or `workflow_dispatch`.
- A feature may move between lanes, but it must never lose its deep lane.
- Nightly failures are tracked to a minimized regression before closure.
