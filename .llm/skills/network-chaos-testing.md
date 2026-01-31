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

## Checklist for New Chaos Tests

- [ ] Network profile has accurate `packet_loss`, `burst_loss_prob`, `burst_loss_len`
- [ ] `suggested_sync_preset()` logic covers the profile's conditions
- [ ] Scenario uses `with_auto_sync_preset()` or explicit `.with_sync_preset()`
- [ ] Unit test verifies the preset selection for the profile
- [ ] Test uses unique port base (≥200 apart from other tests)
- [ ] Timeout is appropriate for the chaos level (higher chaos → longer timeout)
- [ ] Test has been run 10+ times locally to verify it's not flaky
- [ ] Comments document why specific preset/timeout was chosen

---

## References

- [rust-testing-guide.md](rust-testing-guide.md) — General testing best practices
- [determinism-guide.md](determinism-guide.md) — Ensuring deterministic simulation
- [deterministic-simulation-testing.md](deterministic-simulation-testing.md) — DST frameworks

---

*This guide was created based on debugging sync failures in asymmetric chaos scenarios where the combination of high packet loss and burst loss caused sync handshake timeouts.*
