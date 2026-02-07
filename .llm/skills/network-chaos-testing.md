# Network Chaos Testing — Reliable Multi-Peer Session Tests

> **This guide covers best practices for testing networked P2P sessions under simulated adverse network conditions.**
> Use this when adding network chaos tests, diagnosing sync failures, or understanding flaky multi-process tests.

## TL;DR — Quick Reference

```rust
// ✅ CORRECT: Use auto-sync preset for reliable testing
let scenario = NetworkScenario::asymmetric(
    "good_vs_terrible",
    NetworkProfile::wifi_good(),
    NetworkProfile::terrible(),  // 25% loss + 5% burst
)
.with_auto_sync_preset()  // Automatically selects "stress_test"
.with_timeout(180);

// ❌ WRONG: High loss + burst without sync preset → flaky sync failures
let scenario = NetworkScenario::asymmetric(
    "good_vs_terrible",
    NetworkProfile::wifi_good(),
    NetworkProfile::terrible(),  // 25% loss + 5% burst
);  // Missing sync preset → ~60% sync failure rate
```

**Key principles:**

1. **Sync handshake is the vulnerability** — Most chaos test failures occur during initial sync
2. **Bidirectional chaos compounds** — Both peers apply chaos to outgoing packets
3. **Use `with_auto_sync_preset()`** — Let the framework calculate appropriate resilience
4. **Validate scenarios** — Call `validate_sync_preset()` to catch misconfiguration

---

## The Problem: Sync Handshake Vulnerability

The sync handshake in P2P sessions is particularly vulnerable to packet loss because:

1. **Initial sync requires successful round-trips** — Multiple sync packets must succeed
2. **Burst loss can wipe out consecutive sync attempts** — A 5-packet burst at 5% probability can hit during the critical sync window
3. **Default sync config is optimized for good networks** — Low retry counts and short timeouts fail under chaos

### Compounding Effect of Bidirectional Chaos

**Critical insight:** In multi-process chaos tests, each peer applies chaos to its *outgoing* packets independently. This means:

| Peer 1 Outgoing | Peer 2 Outgoing | Effective Bidirectional Loss |
|-----------------|-----------------|------------------------------|
| 25% packet loss | 0% packet loss | ~25% for sync round-trips |
| 25% packet loss | 25% packet loss | ~44% for sync round-trips¹ |
| 25% + 5% burst | 25% + 5% burst | ~60%+ effective loss during bursts |

¹ `1 - (0.75 × 0.75) = 0.4375` — the probability that *either* direction loses a packet

**With burst loss**, the situation is worse: if both peers have 5% burst probability with 5-packet bursts, there's a significant chance of overlapping bursts that completely block sync handshake.

---

## Sync Preset Selection Guidelines

Use this table to select the appropriate sync preset based on network conditions:

| Network Conditions | Sync Preset | Rationale |
|--------------------|-------------|-----------|
| ≤5% packet loss, no burst | `None` (default) | Standard conditions, default is sufficient |
| 5-15% packet loss, no burst | `"lossy"` | Increased retries handle moderate loss |
| 15-20% packet loss, no burst | `"mobile"` | Aggressive retries for unreliable links |
| >20% packet loss, no burst | `"extreme"` | Maximum single-direction resilience |
| Any burst loss (≥2% prob, ≥3 packets) | `"mobile"` | Burst loss needs longer retry windows |
| >20% loss + burst loss | `"stress_test"` | Combination is much worse than either alone |
| >10% burst with ≥8 packet bursts | `"stress_test"` | Extreme burst patterns need maximum resilience |

### Sync Preset Parameters

| Preset | Sync Packets | Retry Interval | Timeout | Use Case |
|--------|-------------|----------------|---------|----------|
| Default | 10 | ~100ms | 10s | LAN, good WiFi |
| `"lossy"` | 20 | ~150ms | 30s | 5-15% packet loss |
| `"mobile"` | 30 | ~200ms | 45s | High jitter, burst loss |
| `"extreme"` | 35 | ~200ms | 60s | >20% packet loss |
| `"stress_test"` | 40 | ~150ms | 60s | Combined high loss + burst |

---

## Defensive Patterns

### 1. Auto-Sync Preset Selection

The `NetworkProfile::suggested_sync_preset()` method analyzes network conditions and recommends an appropriate preset:

```rust
impl NetworkProfile {
    const fn suggested_sync_preset(&self) -> Option<&'static str> {
        // High burst loss with long bursts requires stress_test preset
        if self.burst_loss_prob >= 0.10 && self.burst_loss_len >= 8 {
            return Some("stress_test");
        }

        // Very high packet loss (>20%) combined with burst loss
        if self.packet_loss >= 0.20 && self.burst_loss_prob >= 0.03 {
            return Some("stress_test");
        }

        // High packet loss (>20%) without burst
        if self.packet_loss >= 0.20 {
            return Some("extreme");
        }

        // Significant burst loss
        if self.burst_loss_prob >= 0.02 && self.burst_loss_len >= 3 {
            return Some("mobile");
        }

        // High packet loss (15-20%)
        if self.packet_loss >= 0.15 {
            return Some("mobile");
        }

        // Moderate packet loss (8-15%)
        if self.packet_loss >= 0.08 {
            return Some("lossy");
        }

        None  // Default is fine
    }
}
```

### 2. Use `with_auto_sync_preset()` Builder

```rust
// Automatically applies the appropriate preset for worst-case profile
let scenario = NetworkScenario::asymmetric(
    "good_vs_terrible",
    NetworkProfile::wifi_good(),
    NetworkProfile::terrible(),
)
.with_auto_sync_preset();  // Calculates: terrible → stress_test
```

### 3. Runtime Validation with `validate_sync_preset()`

Scenarios validate themselves at runtime and warn about potential misconfiguration:

```rust
impl NetworkScenario {
    fn validate_sync_preset(&self) {
        let suggested = /* calculate from profiles */;
        if let Some(recommended) = suggested {
            if self.sync_preset.is_none() {
                eprintln!(
                    "⚠️  WARNING: Scenario '{}' has aggressive conditions \
                     but no sync preset. Consider using .with_sync_preset(\"{}\")",
                    self.name, recommended
                );
            }
        }
    }
}
```

### 4. Unit Test the Selection Logic

Test that `suggested_sync_preset()` returns correct values:

```rust
#[test]
fn test_suggested_sync_preset_selection() {
    // Default profile → None
    assert_eq!(NetworkProfile::lan().suggested_sync_preset(), None);

    // 15% loss → mobile
    assert_eq!(
        NetworkProfile::wifi_congested().suggested_sync_preset(),
        Some("mobile")
    );

    // 25% loss + 5% burst → stress_test
    assert_eq!(
        NetworkProfile::terrible().suggested_sync_preset(),
        Some("stress_test")
    );
}
```

---

## Common Failure Modes and Diagnosis

### Failure Mode 1: Sync Timeout at Frame 0

**Symptoms:**

- `success=false, final_frame=0`
- Error: `"Timeout (current_frame=0, confirmed_frame=NULL_FRAME, target=N)"`

**Cause:** Sync handshake never completed due to packet loss during initial exchange.

**Fix:** Add appropriate sync preset using `with_sync_preset()` or `with_auto_sync_preset()`.

### Failure Mode 2: Flaky Test (~50% pass rate)

**Symptoms:**

- Test passes sometimes, fails sometimes
- Failures are always sync timeouts
- `is_sync_timeout_error()` returns `true`

**Cause:** Network conditions are borderline for the sync preset. Some random seeds survive, others don't.

**Fix:**

1. Upgrade to more resilient sync preset
2. Or use `bursty_survivable()` instead of `bursty()` for CI
3. Or add retry logic with `run_test_with_retry()`

### Failure Mode 3: Mid-Session Timeout

**Symptoms:**

- `final_frame > 0` (made progress)
- Error: `"Timeout (current_frame=X, confirmed_frame=Y, target=N)"`

**Cause:** Network chaos disrupted frame confirmation. This is rarer than sync failures and usually indicates the chaos level is too aggressive for the session timeout.

**Fix:** Increase `timeout_secs` or reduce chaos severity.

### Failure Mode 4: Desync After Rollback

**Symptoms:**

- Both peers report `success=true`
- But `final_value` or `checksum` differs between peers

**Cause:** Determinism bug in game logic, not a network issue.

**Fix:** Review game logic for non-determinism (HashMap iteration, floating point, etc.).

---

## How to Add New Network Scenarios

### Step 1: Define the Network Profile (if new)

```rust
/// Profile with specific network characteristics.
const fn my_new_profile() -> Self {
    Self {
        packet_loss: 0.12,
        latency_ms: 40,
        jitter_ms: 20,
        burst_loss_prob: 0.03,
        burst_loss_len: 4,
        // ... other fields
    }
}
```

### Step 2: Verify Suggested Preset

```rust
#[test]
fn test_my_new_profile_preset() {
    let profile = NetworkProfile::my_new_profile();
    // Document expected preset for this profile
    assert_eq!(
        profile.suggested_sync_preset(),
        Some("mobile"),
        "12% loss + 3% burst should use mobile preset"
    );
}
```

### Step 3: Create the Scenario with Auto-Preset

```rust
let scenario = NetworkScenario::asymmetric(
    "my_new_scenario",
    NetworkProfile::lan(),
    NetworkProfile::my_new_profile(),
)
.with_auto_sync_preset()  // Uses suggested_sync_preset()
.with_timeout(120);
```

### Step 4: Add Scenario to Test (Parameterized)

```rust
// In test_asymmetric_scenarios or similar
(
    10400,  // Unique port base
    NetworkScenario::asymmetric(
        "lan_vs_my_new_profile",
        NetworkProfile::lan(),
        NetworkProfile::my_new_profile(),
    )
    .with_frames(80)
    .with_auto_sync_preset(),
),
```

### Step 5: Run and Verify

```bash
# Run the specific test
cargo nextest run test_asymmetric_scenarios --no-capture

# Run multiple times to check for flakiness
for i in {1..10}; do
    cargo nextest run test_asymmetric_scenarios --no-capture || echo "FAILED on run $i"
done
```

---

## Retry Logic for Extreme Chaos

For scenarios with extreme burst loss where even `stress_test` may occasionally fail, use retry logic:

```rust
/// Runs the test with retry logic for sync timeout failures.
fn run_test_with_retry(&self, port_base: u16) -> (TestResult, TestResult, u32) {
    self.run_test_with_retry_config(port_base, DEFAULT_MAX_RETRIES)
}
```

**When to use retries:**

- Scenarios with `bursty()` profile (10% burst, 8-packet bursts)
- Testing edge cases where sync failure is acceptable but we want to eventually pass
- NOT as a substitute for proper sync preset selection

**Anti-pattern:**

```rust
// ❌ WRONG: Using retries to mask missing sync preset
let scenario = NetworkScenario::symmetric("terrible", NetworkProfile::terrible());
scenario.run_test_with_retry(port);  // Will likely need many retries

// ✅ CORRECT: Proper preset, retry only for edge cases
let scenario = NetworkScenario::symmetric("terrible", NetworkProfile::terrible())
    .with_sync_preset("stress_test");
scenario.run_test(port);  // Should pass without retry
```

---

## CI Flakiness Prevention for Network Tests

### The `sync_timeout` vs Iteration Count Problem

**Critical insight:** `sync_timeout` fires based on **wall-clock time**, not iteration count.
On slow CI runners (macOS VMs, shared GitHub Actions runners), each iteration takes
2–3× longer than on a local dev machine. A test that completes in 30s locally may
need 85s on CI — well past a 60s `sync_timeout`.

```rust
// ❌ FRAGILE: sync_timeout is wall-clock dependent
let sync_config = SyncConfig {
    num_sync_packets: 40,
    sync_retry_interval: Duration::from_millis(200),
    sync_timeout: Some(Duration::from_secs(60)),
    running_retry_interval: Duration::from_millis(200),
    keepalive_interval: Duration::from_millis(200),
};
// On a 2.8× slower CI runner, 40 roundtrips may need ~84s — timeout fires at 60s!

// ✅ ROBUST: iteration count is the only budget controller
let sync_config = SyncConfig {
    num_sync_packets: 20,
    sync_retry_interval: Duration::from_millis(200),
    sync_timeout: None,  // Let max_iterations be the budget, not wall-clock
    running_retry_interval: Duration::from_millis(200),
    keepalive_interval: Duration::from_millis(200),
};
```

**Why `sync_timeout: None` is preferred for chaos tests:**

- Chaos tests already have a `max_iterations` budget that bounds execution
- `sync_timeout` adds a **second, non-deterministic** deadline that races against iterations
- Removing it makes test outcome depend only on iteration count — same on every machine
- If the test needs a safety net, use `max_iterations` (deterministic) not `sync_timeout` (wall-clock)

### Budget Headroom Calculation

A test must leave enough iteration budget to absorb CI slowdown.
Use the **2.8× rule**: if a test uses N% of its budget locally, it uses
~2.8×N% on the slowest CI runner.

| Local Budget Usage | CI Usage (×2.8) | Verdict |
|--------------------|----------------|---------|
| ≤50% | ≤140% | ✅ Safe (with margin) |
| 50–70% | 140–196% | ⚠️ Risky — may flake |
| >70% | >196% | ❌ Will flake on slow CI |

**Rule of thumb:** A test should use **≤50% of its iteration budget locally** to leave
room for 2.8× CI slowdown.

```rust
// Example: 20 roundtrips under 30% burst loss
// Locally completes in ~800 iterations out of 2000 max (40% usage)
// On CI: ~800 × 2.8 = ~2240 — still under 2000? NO!
//
// Fix: either reduce roundtrips or increase max_iterations
let max_iterations = 5000;  // 800/5000 = 16% local → ~45% on CI ✅
```

### Reduce Roundtrip Requirements for Harsh Conditions

The probability of completing N consecutive roundtrips under loss is **exponential in N**.
With 30% effective bidirectional loss, the expected attempts to get one successful roundtrip
is ~1.43, but for 40 consecutive successes it becomes astronomically unlikely without
many retries.

| Roundtrips | Relative Difficulty (at 30% loss) | Recommendation |
|------------|-----------------------------------|----------------|
| 10 | Baseline | Good for moderate loss |
| 20 | ~10× harder | Upper limit for harsh conditions |
| 40 | ~100× harder | Too many — reduce or expect failure |

**Guideline:** For burst loss scenarios, use **≤20 roundtrips**. If you need to validate
that sync *works* under extreme conditions, 20 roundtrips is sufficient proof.

### Use Explicit `SyncConfig`, Not Presets, for Chaos Tests

Presets like `SyncConfig::stress_test()` bundle roundtrip counts, retry intervals,
and timeouts that may not align with a chaos test's actual requirements.

```rust
// ❌ FRAGILE: preset has sync_timeout that may not suit this test
let sync_config = SyncConfig::stress_test();  // 40 roundtrips, 60s timeout

// ✅ EXPLICIT: construct exactly what the test needs
let sync_config = SyncConfig {
    num_sync_packets: 20,
    sync_retry_interval: Duration::from_millis(150),
    sync_timeout: None,  // Iteration budget is the only limit
    running_retry_interval: Duration::from_millis(200),
    keepalive_interval: Duration::from_millis(200),
};
```

**Why explicit construction is better:**

- Makes test requirements visible in the test code
- Avoids inheriting preset defaults that cause flakiness
- Each chaos scenario may need different roundtrip/timeout combinations
- Preset changes won't silently break unrelated tests

### Include Expected-Failure Test Cases

Data-driven chaos tests should include cases where sync is **expected to fail**.
This validates the test infrastructure and provides confidence that success cases
are meaningful.

```rust
// ✅ GOOD: test both success and expected-failure conditions
let cases = vec![
    // Moderate loss — should succeed
    ChaosCase {
        name: "moderate_loss",
        profile: NetworkProfile::wifi_congested(),
        expect_sync: true,
    },
    // Extreme burst loss — should fail to sync (validates infrastructure)
    ChaosCase {
        name: "high_burst_loss",
        profile: NetworkProfile::extreme_burst(),
        expect_sync: false,  // Confirms chaos is actually applied
    },
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

**Why expected-failure cases matter:**

- If *everything* passes, your chaos might not be applied
- Expected failures prove the chaos socket is working
- They catch regressions where error handling silently changes
- They document the boundary between survivable and unsurvivable conditions

---

## Checklist for New Chaos Tests

- [ ] Network profile has accurate `packet_loss`, `burst_loss_prob`, `burst_loss_len`
- [ ] `suggested_sync_preset()` logic covers the profile's conditions
- [ ] Scenario uses `with_auto_sync_preset()` or explicit `.with_sync_preset()`
- [ ] Unit test verifies the preset selection for the profile
- [ ] Test uses unique port base (≥200 apart from other tests)
- [ ] Timeout is appropriate for the chaos level (higher chaos → longer timeout)
- [ ] Test has been run 10+ times locally to verify it's not flaky
- [ ] Comments document why specific preset/timeout was chosen
- [ ] Uses `sync_timeout: None` (iteration budget is the only limit)
- [ ] Uses explicit `SyncConfig` construction, not presets
- [ ] Local budget usage is ≤50% (leaves room for 2.8× CI slowdown)
- [ ] Roundtrip count is ≤20 for harsh loss/burst conditions
- [ ] Includes expected-failure cases for extreme conditions

---

## References

- [rust-testing-guide.md](rust-testing-guide.md) — General testing best practices
- [determinism-guide.md](determinism-guide.md) — Ensuring deterministic simulation
- [deterministic-simulation-testing.md](deterministic-simulation-testing.md) — DST frameworks

---

*This guide was created based on debugging sync failures in asymmetric chaos scenarios where the combination of high packet loss and burst loss caused sync handshake timeouts.*
