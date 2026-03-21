<!-- CATEGORY: Workflows -->
<!-- WHEN: Security review, safety audit, adversarial testing, red-team analysis of changes -->

# Adversarial Review Guide

Challenge every change by asking: "How could this fail in production?" For a rollback networking library, failures mean desyncs, corrupted state, or denial-of-service in multiplayer games. Review with the assumption that inputs are adversarial, networks are hostile, and peers are untrusted.

---

## Inversion Reflex

For every claim in a PR, invert it:

| Claim | Inversion Question |
|-------|-------------------|
| "This is safe" | What input makes it panic or overflow? |
| "This is deterministic" | What platform or config makes it diverge? |
| "This handles all errors" | What error path was not tested? |
| "This is backward compatible" | What existing code breaks? |
| "Performance is fine" | What happens at 1000 players or 10000 frames? |
| "The test proves it works" | What does the test NOT cover? |

---

## Threat Model for Rollback Netcode

### Untrusted Peer Inputs

Every UDP packet from a remote peer is untrusted. Check:

```rust
// Does the code validate frame numbers from network messages?
// A malicious peer could send:
// - frame = i32::MAX (overflow on arithmetic)
// - frame = -1 (NULL_FRAME in unexpected context)
// - frame = 0 when session is at frame 10000 (ancient frame)
// - player_handle >= num_players (out of bounds, zero-indexed)
```

Verify all network-received values are validated before use:

| Field | Validation Required |
|-------|-------------------|
| `frame` | `>= 0`, within prediction window of current frame |
| `player_handle` | `< num_players`, matches expected sender |
| `input_size` | Matches `Config::Input` size exactly |
| `sequence_number` | Within reasonable window of last seen (replay protection) |
| `ack_frame` | `>= 0`, `<= remote's current frame` |

### Denial of Service

A malicious peer could try to:

- Send no inputs (force prediction threshold)
- Send conflicting inputs for the same frame
- Send inputs far in the future (memory exhaustion)
- Flood with sync requests
- Never acknowledge frames (prevent garbage collection)

For each, verify the code has bounds:

```bash
# Check that input queues have bounded size
rg 'Vec::new\(\)|Vec::with_capacity|VecDeque::new' --type rust src/input_queue/
# Should use fixed-size circular buffers, not unbounded collections
```

### State Corruption

Rollback requires loading old state. What if:

- `SaveGameState` is called but game does not save?
- `LoadGameState` loads state from wrong frame?
- Circular buffer index wraps incorrectly?
- State is partially saved (some fields, not all)?

---

## Determinism Attack Surface

Non-determinism causes desyncs, which are catastrophic in multiplayer. Audit every line in the diff for:

### Floating-Point Hazards

```bash
# Search for float operations in simulation code
rg 'f32|f64' --type rust src/sync_layer/ src/input_queue/ src/sessions/
```

Any float comparison, accumulation, or transcendental function is suspect. Verify `libm` is used or fixed-point math is in place.

### Collection Ordering

```bash
# Any new HashMap/HashSet usage?
rg 'HashMap|HashSet' --type rust src/
# Any iteration that depends on insertion order?
rg '\.iter\(\)|\.into_iter\(\)|for .* in' --type rust src/
```

### Platform-Dependent Behavior

```bash
# usize depends on platform (32 vs 64 bit)
rg 'as usize|usize' --type rust src/
# Verify explicit u32/u64/i32 instead of usize in protocol code
```

---

## Overflow and Arithmetic Review

Fortress uses `Frame` (wrapping `i32`). Every arithmetic operation is a potential overflow:

```bash
# Rough heuristic: find raw arithmetic on numeric types (may have false positives)
rg '\+ \d|\- \d|\* \d' --type rust src/
# Verify checked operations are used instead
rg 'try_add|try_sub|checked_add|checked_sub|safe_frame_add' --type rust src/
```

### Specific Overflow Scenarios

| Operation | Overflow When | Impact |
|-----------|--------------|--------|
| `frame + max_prediction` | `frame` near `i32::MAX` | Wrong rollback bound |
| `frame - rollback_depth` | `frame` near `i32::MIN` | Negative frame index |
| `queue_index % length` | Negative dividend | Wrong buffer slot |
| `num_players * input_size` | Large player count | Buffer overallocation |

---

## Error Path Audit

### Verify Error Paths Are Tested

For every `return Err(...)` or `?` operator in the diff:

1. Is there a test that triggers this exact error path?
2. Does the error variant contain enough context to debug?
3. Is the error recoverable, or should the session terminate?

### Verify Errors Are Not Swallowed

```bash
# Search for discarded Results
rg 'let _ =' --type rust src/
# Search for unwrap_or_default that hides failures
rg 'unwrap_or_default\(\)|unwrap_or\(' --type rust src/
```

Every `let _ = fallible_op()` in production code needs justification.

---

## Backward Compatibility

### Wire Protocol

If the change modifies anything in `src/network/`:

- Can a new client talk to an old client?
- Can an old client talk to a new client?
- Are new message fields defaulted for old peers?

### Public API

```bash
# Check for removed or changed pub items
rg 'pub fn|pub struct|pub enum|pub trait|pub type' --type rust src/
```

Removed or renamed public items need `#[deprecated]` first, then removal in a major version.

### Config Defaults

If a default value changes (e.g., `DEFAULT_MAX_PREDICTION_FRAMES`), existing users get different behavior without changing their code. This is a silent breaking change.

---

## Red Team Checklist

For each item, mark PASS, FAIL, or N/A:

- [ ] All network-received values validated before use
- [ ] No unbounded allocations from untrusted input
- [ ] Frame arithmetic uses checked operations
- [ ] No `HashMap`/`HashSet` in simulation paths
- [ ] No floating-point in deterministic code paths
- [ ] No `usize` in wire protocol (use explicit-width types)
- [ ] Error paths tested and not swallowed
- [ ] Circular buffer indices cannot wrap incorrectly
- [ ] State save/load is complete (no missing fields)
- [ ] No silent behavior changes from default value changes
- [ ] Backward-compatible wire protocol (or documented break)
- [ ] Panic-free under all inputs (including adversarial)
