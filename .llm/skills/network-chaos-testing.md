<!-- CATEGORY: Testing -->
<!-- WHEN: Testing network resilience, chaos testing, sync failure diagnosis -->

# Network Chaos Testing

## Quick Reference

```rust
// CORRECT: Use auto-sync preset for reliable testing
let scenario = NetworkScenario::asymmetric(
    "good_vs_terrible",
    NetworkProfile::wifi_good(),
    NetworkProfile::terrible(),
)
.with_auto_sync_preset()
.with_timeout(180);

// WRONG: High loss + burst without sync preset = flaky sync failures
let scenario = NetworkScenario::asymmetric(
    "good_vs_terrible",
    NetworkProfile::wifi_good(),
    NetworkProfile::terrible(),
);  // Missing sync preset = ~60% sync failure rate
```

**Key principles:**
1. Sync handshake is the vulnerability -- most chaos test failures occur during initial sync
2. Bidirectional chaos compounds -- both peers apply chaos to outgoing packets
3. Use `with_auto_sync_preset()` -- let the framework calculate appropriate resilience
4. Use explicit `SyncConfig` construction, not presets, for chaos tests
5. Use `sync_timeout: None` -- let iteration budget be the only limit

## Compounding Effect of Bidirectional Loss

| Peer 1 Out | Peer 2 Out | Effective Bidirectional Loss |
|------------|------------|------------------------------|
| 25% loss | 0% loss | ~25% |
| 25% loss | 25% loss | ~44% (`1 - 0.75 * 0.75`) |
| 25% + 5% burst | 25% + 5% burst | ~60%+ during bursts |

## Sync Preset Selection

| Network Conditions | Sync Preset |
|--------------------|-------------|
| <=5% loss, no burst | None (default) |
| 5-15% loss, no burst | `"lossy"` |
| 15-20% loss, no burst | `"mobile"` |
| >20% loss, no burst | `"extreme"` |
| Any burst loss (>=2%, >=3 packets) | `"mobile"` |
| >20% loss + burst | `"stress_test"` |
| >10% burst with >=8 packet bursts | `"stress_test"` |

### Preset Parameters

| Preset | Sync Packets | Retry Interval | Timeout |
|--------|-------------|----------------|---------|
| Default | 10 | ~100ms | 10s |
| `"lossy"` | 20 | ~150ms | 30s |
| `"mobile"` | 30 | ~200ms | 45s |
| `"extreme"` | 35 | ~200ms | 60s |
| `"stress_test"` | 40 | ~150ms | 60s |

## CI Flakiness Prevention

### The `sync_timeout` vs Iteration Count Problem

`sync_timeout` fires based on **wall-clock time**, not iterations. On slow CI runners (2.8x slower), tests that pass locally will flake.

```rust
// FRAGILE: wall-clock dependent
let sync_config = SyncConfig {
    sync_timeout: Some(Duration::from_secs(60)),  // Races against iterations
    ..
};

// ROBUST: iteration count is the only budget controller
let sync_config = SyncConfig {
    num_sync_packets: 20,
    sync_retry_interval: Duration::from_millis(150),
    sync_timeout: None,  // Let max_iterations be the budget
    running_retry_interval: Duration::from_millis(200),
    keepalive_interval: Duration::from_millis(200),
};
```

### Budget Headroom (2.8x Rule)

| Local Budget Usage | CI Usage (x2.8) | Verdict |
|--------------------|-----------------|---------|
| <=50% | <=140% | Safe |
| 50-70% | 140-196% | Risky |
| >70% | >196% | Will flake |

Rule: A test should use **<=50% of its iteration budget locally**.

### Roundtrip Count Limits

| Roundtrips | Recommendation (at 30% loss) |
|-----------|------------------------------|
| 10 | Good for moderate loss |
| 20 | Upper limit for harsh conditions |
| 40 | Too many -- reduce or expect failure |

## Failure Mode Diagnosis

### Frame 0 Sync Timeout
- Symptom: `success=false, final_frame=0`
- Cause: Sync handshake never completed
- Fix: Add appropriate sync preset

### Flaky Test (~50% pass rate)
- Symptom: Passes sometimes, always sync timeouts when failing
- Fix: Upgrade sync preset, or use `bursty_survivable()` for CI

### Mid-Session Timeout
- Symptom: `final_frame > 0` (made progress)
- Fix: Increase `timeout_secs` or reduce chaos severity

### Desync After Rollback
- Symptom: Both peers `success=true` but values/checksums differ
- Cause: Determinism bug (HashMap iteration, floating point, etc.)
- Fix: Review game logic, not network config

## Adding New Scenarios

```rust
// 1. Define profile
const fn my_profile() -> Self {
    Self { packet_loss: 0.12, latency_ms: 40, jitter_ms: 20,
           burst_loss_prob: 0.03, burst_loss_len: 4, .. }
}

// 2. Verify preset selection
#[test]
fn test_my_profile_preset() {
    assert_eq!(NetworkProfile::my_profile().suggested_sync_preset(), Some("mobile"));
}

// 3. Create scenario with auto-preset
let scenario = NetworkScenario::asymmetric("my_scenario",
    NetworkProfile::lan(), NetworkProfile::my_profile(),
).with_auto_sync_preset().with_timeout(120);

// 4. Run 10+ times locally to verify not flaky
```

## Include Expected-Failure Cases

```rust
let cases = vec![
    ChaosCase { name: "moderate_loss", profile: NetworkProfile::wifi_congested(),
                expect_sync: true },
    ChaosCase { name: "high_burst_loss", profile: NetworkProfile::extreme_burst(),
                expect_sync: false },  // Validates chaos is actually applied
];
for case in &cases {
    let result = run_chaos_test(case);
    if case.expect_sync {
        assert!(result.synced, "{}: expected sync success", case.name);
    } else {
        assert!(!result.synced, "{}: expected sync failure", case.name);
    }
}
```

## Checklist for New Chaos Tests

- [ ] Network profile has accurate loss/burst parameters
- [ ] `suggested_sync_preset()` logic covers the profile
- [ ] Scenario uses `with_auto_sync_preset()` or explicit `.with_sync_preset()`
- [ ] Unit test verifies preset selection for the profile
- [ ] Unique port base (>=200 apart from other tests)
- [ ] Uses `sync_timeout: None` (iteration budget is the only limit)
- [ ] Uses explicit `SyncConfig` construction, not presets
- [ ] Local budget usage <=50% (leaves room for 2.8x CI slowdown)
- [ ] Roundtrip count <=20 for harsh loss/burst conditions
- [ ] Includes expected-failure cases for extreme conditions
- [ ] Test run 10+ times locally to verify not flaky
