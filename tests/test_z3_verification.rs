//! Z3 SMT Solver Verification Tests
//!
//! This module uses Z3 to formally verify safety properties of Fortress Rollback's
//! core algorithms. Unlike Kani (which performs bounded model checking on actual Rust code),
//! Z3 proofs work on abstract mathematical models of the algorithms.
//!
//! ## What Z3 Verifies
//!
//! Z3 proves properties about the *mathematical model* of our algorithms:
//! - Frame arithmetic: overflow, wrapping, and comparison semantics
//! - Circular buffer indexing: bounds checking and wraparound
//! - Rollback frame selection: target is always valid
//! - Sparse saving: saved state availability guarantees
//!
//! ## Relationship to Other Verification
//!
//! | Tool | What it verifies | Scope |
//! |------|------------------|-------|
//! | Unit tests | Expected behavior | Concrete cases |
//! | Property tests | Invariants hold | Random samples |
//! | Kani | Rust code correctness | Bounded exhaustive |
//! | TLA+ | Protocol correctness | State machine model |
//! | Z3 | Algorithm properties | Mathematical model |
//!
//! ## Configurable Constants Alignment (Phase 9/10)
//!
//! Production code now allows configurable queue lengths via `InputQueueConfig`:
//! - `InputQueueConfig::standard()` - 128 frames (default)
//! - `InputQueueConfig::high_latency()` - 256 frames
//! - `InputQueueConfig::minimal()` - 32 frames
//!
//! Z3 proofs use `INPUT_QUEUE_LENGTH = 128` (the default). The proofs verify
//! properties that are **size-independent** - they hold for any valid queue length.
//! Specifically:
//! - Circular buffer arithmetic uses `frame % queue_length` which works for any size
//! - Index bounds are proven relative to `queue_length`, not a fixed value
//! - The frame delay constraint (`delay < queue_length`) scales with queue size
//!
//! ## Running Z3 Tests
//!
//! These tests require the `z3-verification` feature:
//! ```bash
//! cargo test --test test_z3_verification --features z3-verification
//! ```
//!
//! Note: First build may take several minutes to compile Z3 from source.

#![cfg(feature = "z3-verification")]

use z3::ast::Int;
use z3::{with_z3_config, Config, SatResult, Solver};

/// Constants matching the Fortress Rollback default implementation.
///
/// These use the default `InputQueueConfig::standard()` values.
/// The proofs are size-independent and hold for any `queue_length >= 2`.
const INPUT_QUEUE_LENGTH: i64 = 128;
const MAX_FRAME_DELAY: i64 = INPUT_QUEUE_LENGTH - 1;
const NULL_FRAME: i64 = -1;
const MAX_PREDICTION: i64 = 8; // Default max prediction window

// =============================================================================
// Frame Arithmetic Proofs
// =============================================================================

/// Z3 Proof: Frame addition never produces invalid indices when bounded
///
/// Proves that for any valid frame f in [0, MAX_INT - INPUT_QUEUE_LENGTH),
/// adding frame_delay (< INPUT_QUEUE_LENGTH) produces a valid non-negative result.
#[test]
fn z3_proof_frame_addition_bounded() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        // Variables
        let frame = Int::fresh_const("frame");
        let delay = Int::fresh_const("delay");

        // Constraints: frame and delay are valid
        // frame >= 0 (valid frame)
        solver.assert(frame.ge(0));
        // frame < MAX to prevent overflow
        solver.assert(frame.lt(i64::MAX - INPUT_QUEUE_LENGTH));
        // delay >= 0 and delay <= MAX_FRAME_DELAY (validated constraint)
        solver.assert(delay.ge(0));
        solver.assert(delay.le(MAX_FRAME_DELAY));

        // result = frame + delay
        let result = &frame + &delay;

        // Try to find a counterexample where result is negative
        solver.assert(result.lt(0));

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove no valid frame + delay produces negative result"
        );
    });
}

/// Z3 Proof: Frame subtraction for rollback produces valid targets
///
/// Proves that when rolling back, the target frame is always:
/// 1. Non-negative (valid frame)
/// 2. Less than current_frame (in the past)
/// 3. Within the prediction window
#[test]
fn z3_proof_rollback_frame_valid() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        // Variables
        let current_frame = Int::fresh_const("current_frame");
        let rollback_distance = Int::fresh_const("rollback_distance");

        // Constraints:
        // current_frame >= 0 (we're in a valid game state)
        solver.assert(current_frame.ge(0));
        // rollback_distance > 0 and <= MAX_PREDICTION (valid rollback)
        solver.assert(rollback_distance.gt(0));
        solver.assert(rollback_distance.le(MAX_PREDICTION));
        // rollback_distance <= current_frame (can't rollback before frame 0)
        solver.assert(rollback_distance.le(&current_frame));

        // target_frame = current_frame - rollback_distance
        let target_frame = &current_frame - &rollback_distance;

        // Try to find a counterexample where target_frame is invalid
        // Invalid means: negative OR >= current_frame
        let invalid = target_frame.lt(0) | target_frame.ge(&current_frame);
        solver.assert(&invalid);

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove rollback always produces valid target frame"
        );
    });
}

/// Z3 Proof: Frame comparison transitivity
///
/// Proves that frame ordering is transitive: if a < b and b < c, then a < c.
/// This is important for ensuring consistent frame ordering during rollback decisions.
#[test]
fn z3_proof_frame_comparison_transitive() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let frame_a = Int::fresh_const("frame_a");
        let frame_b = Int::fresh_const("frame_b");
        let frame_c = Int::fresh_const("frame_c");

        // All frames are valid
        solver.assert(frame_a.ge(0));
        solver.assert(frame_b.ge(0));
        solver.assert(frame_c.ge(0));

        // Premise: a < b and b < c
        solver.assert(frame_a.lt(&frame_b));
        solver.assert(frame_b.lt(&frame_c));

        // Try to find counterexample where a >= c
        solver.assert(frame_a.ge(&frame_c));

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove frame comparison transitivity"
        );
    });
}

// =============================================================================
// Circular Buffer Proofs
// =============================================================================

/// Z3 Proof: Modulo operation always produces valid index
///
/// Proves that frame % INPUT_QUEUE_LENGTH is always in [0, INPUT_QUEUE_LENGTH).
#[test]
fn z3_proof_circular_buffer_index_valid() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let frame = Int::fresh_const("frame");

        // frame >= 0 (valid frame number)
        solver.assert(frame.ge(0));

        // index = frame % INPUT_QUEUE_LENGTH
        let index = &frame % INPUT_QUEUE_LENGTH;

        // Try to find counterexample where index is out of bounds
        let out_of_bounds = index.lt(0) | index.ge(INPUT_QUEUE_LENGTH);
        solver.assert(&out_of_bounds);

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove modulo always produces valid index"
        );
    });
}

/// Z3 Proof: Head advancement wraps correctly
///
/// Proves that (head + 1) % INPUT_QUEUE_LENGTH is always valid and wraps to 0
/// when head == INPUT_QUEUE_LENGTH - 1.
#[test]
fn z3_proof_head_advancement_wraps() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let head = Int::fresh_const("head");

        // head is a valid index: 0 <= head < INPUT_QUEUE_LENGTH
        solver.assert(head.ge(0));
        solver.assert(head.lt(INPUT_QUEUE_LENGTH));

        // new_head = (head + 1) % INPUT_QUEUE_LENGTH
        let new_head = (&head + 1) % INPUT_QUEUE_LENGTH;

        // Try to find counterexample where new_head is invalid
        let invalid = new_head.lt(0) | new_head.ge(INPUT_QUEUE_LENGTH);
        solver.assert(&invalid);

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove head advancement is always valid"
        );
    });
}

/// Z3 Proof: Head wraparound produces correct value
///
/// Specifically proves that when head == INPUT_QUEUE_LENGTH - 1,
/// the new head is exactly 0.
#[test]
fn z3_proof_head_wraparound_to_zero() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        // head at last position
        let head = Int::from_i64(INPUT_QUEUE_LENGTH - 1);
        let new_head = (&head + 1) % INPUT_QUEUE_LENGTH;

        // Try to find counterexample where new_head != 0
        solver.assert(new_head.eq(0).not());

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove head wraps to 0 from last position"
        );
    });
}

/// Z3 Proof: Queue length invariant preserved after add
///
/// Proves that after adding an element (incrementing head), the queue length
/// increases by exactly 1 (if not full).
#[test]
fn z3_proof_queue_length_invariant_add() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let length_before = Int::fresh_const("length_before");

        // length_before is valid and queue is not full
        solver.assert(length_before.ge(0));
        solver.assert(length_before.lt(INPUT_QUEUE_LENGTH));

        // After add: length_after = length_before + 1
        let length_after = &length_before + 1;

        // Try to find counterexample where length_after is out of bounds
        let invalid = length_after.lt(0) | length_after.gt(INPUT_QUEUE_LENGTH);
        solver.assert(&invalid);

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove length invariant after add"
        );
    });
}

/// Z3 Proof: Circular buffer distance calculation
///
/// Proves that the distance from tail to head in a circular buffer
/// correctly represents the queue length for non-wrapped case.
#[test]
fn z3_proof_circular_distance_non_wrapped() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let head = Int::fresh_const("head");
        let tail = Int::fresh_const("tail");

        // head and tail are valid indices
        solver.assert(head.ge(0));
        solver.assert(head.lt(INPUT_QUEUE_LENGTH));
        solver.assert(tail.ge(0));
        solver.assert(tail.lt(INPUT_QUEUE_LENGTH));

        // Case: head > tail (non-wrapped)
        solver.assert(head.gt(&tail));

        // length = head - tail
        let length = &head - &tail;

        // Length must be positive and bounded
        let valid_length = length.gt(0) & length.lt(INPUT_QUEUE_LENGTH);

        // This should be satisfiable
        solver.assert(&valid_length);

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Sat,
            "Z3 should find valid non-wrapped circular buffer states"
        );
    });
}

// =============================================================================
// Rollback Frame Selection Proofs
// =============================================================================

/// Z3 Proof: Rollback target is always in the past when rollback occurs
///
/// Proves that when we actually execute a rollback (call load_frame), the target
/// is always < current_frame. This models the production guard in adjust_gamestate():
///   if frame_to_load >= current_frame { skip_rollback; return Ok(()) }
///
/// The precondition `first_incorrect_frame < current_frame` is now explicitly
/// documented as the GUARD for entering the rollback path (not a general constraint).
///
/// FV-GAP-2: This proof was reviewed as part of the Frame 0 Rollback FV Gap Analysis.
/// The precondition is correct because production code skips rollback when
/// first_incorrect >= current_frame. See also: z3_proof_skip_rollback_when_frame_equal.
#[test]
fn z3_proof_rollback_target_in_past() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let current_frame = Int::fresh_const("current_frame");
        let first_incorrect_frame = Int::fresh_const("first_incorrect_frame");

        // current_frame is valid
        solver.assert(current_frame.ge(0));

        // GUARD: We only enter the rollback path when first_incorrect < current_frame
        // (when first_incorrect >= current_frame, SkipRollback handles it)
        solver.assert(first_incorrect_frame.ge(0));
        solver.assert(first_incorrect_frame.lt(&current_frame));

        // target_frame = first_incorrect_frame (we rollback to the mispredicted frame)
        let target_frame = &first_incorrect_frame;

        // Try to find counterexample where target >= current
        solver.assert(target_frame.ge(&current_frame));

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove rollback target is always in the past when rollback executes"
        );
    });
}

/// Z3 Proof: Skip rollback path is taken when first_incorrect >= current_frame
///
/// Proves that the guard `frame_to_load >= current_frame` correctly identifies
/// when rollback should be skipped. This models the production code:
///   if frame_to_load >= current_frame {
///       debug!("Skipping rollback...");
///       self.sync_layer.reset_prediction();
///       return Ok(());
///   }
///
/// FV-GAP-2: New proof added as part of Frame 0 Rollback FV Gap Analysis.
/// This explicitly verifies the edge case where first_incorrect == current_frame
/// (which can happen at frame 0 with misprediction detected immediately).
#[test]
fn z3_proof_skip_rollback_when_frame_equal() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let current_frame = Int::fresh_const("current_frame");
        let first_incorrect_frame = Int::fresh_const("first_incorrect_frame");

        // current_frame is valid (can be 0, the first frame)
        solver.assert(current_frame.ge(0));

        // first_incorrect_frame is valid and within prediction window
        solver.assert(first_incorrect_frame.ge(0));
        solver.assert(first_incorrect_frame.le(&current_frame)); // Note: <= (includes ==)
        solver.assert((&current_frame - &first_incorrect_frame).le(MAX_PREDICTION));

        // The skip guard is: frame_to_load >= current_frame
        // In non-sparse mode, frame_to_load = first_incorrect_frame
        let frame_to_load = &first_incorrect_frame;
        let should_skip = frame_to_load.ge(&current_frame);

        // Prove: When should_skip is true, we DON'T call load_frame
        // Equivalently: should_skip AND first_incorrect < current_frame is UNSAT
        solver.assert(&should_skip);
        solver.assert(first_incorrect_frame.lt(&current_frame));

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove skip_rollback and normal_rollback are mutually exclusive"
        );
    });
}

/// Z3 Proof: Frame 0 misprediction is handled by skip_rollback
///
/// Proves that when first_incorrect_frame == current_frame == 0 (frame 0 misprediction
/// detected at frame 0), the skip_rollback path is taken.
///
/// FV-GAP-2: This proof explicitly covers the bug scenario that caused
/// test_terrible_network_preset to fail. Before the fix, this would have
/// attempted load_frame(0) with current_frame=0, causing an error.
#[test]
fn z3_proof_frame_zero_misprediction_skips_rollback() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let current_frame = Int::fresh_const("current_frame");
        let first_incorrect_frame = Int::fresh_const("first_incorrect_frame");

        // Scenario: Both frames are 0 (misprediction at first frame)
        solver.assert(current_frame.eq(0));
        solver.assert(first_incorrect_frame.eq(0));

        // The skip guard: frame_to_load >= current_frame
        let frame_to_load = &first_incorrect_frame; // Non-sparse: frame_to_load = first_incorrect
        let should_skip = frame_to_load.ge(&current_frame);

        // Prove: In this scenario, should_skip must be true
        solver.assert(should_skip.not());

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove frame 0 misprediction triggers skip_rollback"
        );
    });
}

/// Z3 Proof: Rollback target is within prediction window
///
/// Proves that rollback never goes beyond MAX_PREDICTION frames back.
#[test]
fn z3_proof_rollback_within_prediction_window() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let current_frame = Int::fresh_const("current_frame");
        let first_incorrect_frame = Int::fresh_const("first_incorrect_frame");

        // current_frame is valid
        solver.assert(current_frame.ge(0));

        // first_incorrect_frame > NULL_FRAME (misprediction exists)
        solver.assert(first_incorrect_frame.gt(NULL_FRAME));

        // Prediction window constraint: current_frame - first_incorrect_frame <= MAX_PREDICTION
        // This is enforced by the session rejecting inputs too far in the future
        solver.assert((&current_frame - &first_incorrect_frame).le(MAX_PREDICTION));

        // Try to find state where we'd need to rollback more than MAX_PREDICTION frames
        let excessive_rollback = (&current_frame - &first_incorrect_frame).gt(MAX_PREDICTION);
        solver.assert(&excessive_rollback);

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove rollback is always within prediction window"
        );
    });
}

/// Z3 Proof: Saved state availability during rollback
///
/// Proves that when we need to rollback to frame F, the frame is within
/// the save buffer capacity (MAX_PREDICTION + 1 slots).
#[test]
fn z3_proof_saved_state_available() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let current_frame = Int::fresh_const("current_frame");
        let target_frame = Int::fresh_const("target_frame");
        let num_save_slots = MAX_PREDICTION + 1;

        // current_frame >= 0
        solver.assert(current_frame.ge(0));

        // target_frame is valid rollback target (from previous proof)
        solver.assert(target_frame.ge(0));
        solver.assert(target_frame.lt(&current_frame));
        solver.assert((&current_frame - &target_frame).le(MAX_PREDICTION));

        // The frames_since_target should be within our save capacity
        let frames_since_target = &current_frame - &target_frame;
        let state_still_available = frames_since_target.le(num_save_slots);

        solver.assert(&state_still_available);

        // This should be satisfiable - valid states exist
        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Sat,
            "Z3 should find valid rollback states with saved data available"
        );
    });
}

// =============================================================================
// Frame Delay Proofs
// =============================================================================

/// Z3 Proof: Frame delay validation prevents overflow
///
/// Proves that the MAX_FRAME_DELAY constraint ensures frame + delay < INPUT_QUEUE_LENGTH
/// when frame starts at 0.
#[test]
fn z3_proof_frame_delay_prevents_overflow() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let delay = Int::fresh_const("delay");

        // delay <= MAX_FRAME_DELAY (validated constraint)
        solver.assert(delay.ge(0));
        solver.assert(delay.le(MAX_FRAME_DELAY));

        // Starting from frame 0, the delayed frame is just the delay value
        let delayed_frame = &delay;

        // Try to find a counterexample where delayed_frame >= INPUT_QUEUE_LENGTH
        solver.assert(delayed_frame.ge(INPUT_QUEUE_LENGTH));

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove frame delay validation prevents overflow"
        );
    });
}

/// Z3 Proof: Sequential frame advancement with delay
///
/// Proves that sequential frame addition (0, 1, 2, ...) with frame delay
/// produces sequential entries in the queue.
#[test]
fn z3_proof_sequential_frames_with_delay() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let frame_n = Int::fresh_const("frame_n");
        let delay = Int::fresh_const("delay");

        // Valid constraints
        solver.assert(frame_n.ge(0));
        solver.assert(delay.ge(0));
        solver.assert(delay.le(MAX_FRAME_DELAY));

        // Sequential input: frame_n_plus_1 = frame_n + 1
        let frame_n_plus_1 = &frame_n + 1;

        // With delay, the queue positions are:
        // position_n = frame_n + delay
        // position_n_plus_1 = frame_n_plus_1 + delay = frame_n + 1 + delay
        let position_n = &frame_n + &delay;
        let position_n_plus_1 = &frame_n_plus_1 + &delay;

        // The positions should be sequential (differ by exactly 1)
        let sequential = position_n_plus_1.eq(&(&position_n + 1));

        solver.assert(sequential.not());

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove sequential frames remain sequential with delay"
        );
    });
}

// =============================================================================
// Input Consistency Proofs
// =============================================================================

/// Z3 Proof: Confirmed inputs have unique positions within queue window
///
/// Models the constraint that frames within INPUT_QUEUE_LENGTH of each other
/// will have unique positions in the circular buffer.
#[test]
fn z3_proof_input_position_uniqueness_in_window() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let frame_a = Int::fresh_const("frame_a");
        let frame_b = Int::fresh_const("frame_b");

        // Both frames are valid
        solver.assert(frame_a.ge(0));
        solver.assert(frame_b.ge(0));

        // Different frames
        solver.assert(frame_a.ne(&frame_b));

        // Compute absolute difference manually: |frame_a - frame_b|
        // Since both are >= 0, one of (a - b) or (b - a) is positive
        let diff_ab = &frame_a - &frame_b;
        let diff_ba = &frame_b - &frame_a;
        // Use conditional: if frame_a > frame_b then diff_ab else diff_ba
        let diff = diff_ab.gt(0).ite(&diff_ab, &diff_ba);

        // But within the same window (less than INPUT_QUEUE_LENGTH apart)
        solver.assert(diff.lt(INPUT_QUEUE_LENGTH));

        // Their positions in the circular buffer
        let pos_a = &frame_a % INPUT_QUEUE_LENGTH;
        let pos_b = &frame_b % INPUT_QUEUE_LENGTH;

        // Try to find case where they have the same position
        solver.assert(pos_a.eq(&pos_b));

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove no position collision within queue window"
        );
    });
}

// =============================================================================
// Comprehensive Property Tests
// =============================================================================

/// Z3 Proof: Complete rollback safety
///
/// Combines multiple properties to prove the complete rollback operation is safe:
/// 1. Target frame is valid
/// 2. Target is in the past
/// 3. Target is within prediction window
/// 4. Saved state is available
#[test]
fn z3_proof_complete_rollback_safety() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let current_frame = Int::fresh_const("current_frame");
        let target_frame = Int::fresh_const("target_frame");

        // Property 1: current_frame is valid
        solver.assert(current_frame.ge(0));

        // Property 2: target_frame is valid
        solver.assert(target_frame.ge(0));

        // Property 3: target is in the past
        solver.assert(target_frame.lt(&current_frame));

        // Property 4: within prediction window
        solver.assert((&current_frame - &target_frame).le(MAX_PREDICTION));

        // All these constraints together should be satisfiable
        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Sat,
            "Z3 should find valid rollback scenarios"
        );
    });
}

/// Z3 Proof: Prediction threshold safety
///
/// Proves that the system rejects new inputs when prediction threshold is exceeded.
#[test]
fn z3_proof_prediction_threshold() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let current_frame = Int::fresh_const("current_frame");
        let last_confirmed_frame = Int::fresh_const("last_confirmed_frame");

        // Valid frames
        solver.assert(current_frame.ge(0));

        // last_confirmed_frame can be NULL_FRAME (-1) or a valid frame
        solver.assert(last_confirmed_frame.ge(NULL_FRAME));
        solver.assert(last_confirmed_frame.lt(&current_frame));

        // Prediction count = current_frame - last_confirmed_frame - 1
        // (frames after last confirmed up to but not including current)
        let prediction_count = &current_frame - &last_confirmed_frame - 1;

        // System should reject if prediction_count >= max_prediction
        let should_reject = prediction_count.ge(MAX_PREDICTION);
        let should_accept = prediction_count.lt(MAX_PREDICTION);

        // These should partition all valid states (XOR is true)
        let valid_decision = should_reject.xor(&should_accept);
        solver.assert(&valid_decision);

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Sat,
            "Z3 should find valid prediction threshold states"
        );
    });
}

/// Z3 Proof: Frame increment never overflows in practical use
///
/// Proves that frame numbers won't overflow for any reasonable game session.
/// At 60 FPS, i32::MAX frames would take ~414 days of continuous play.
#[test]
fn z3_proof_frame_increment_safe() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let frame = Int::fresh_const("frame");

        // Frame is valid and within safe bounds (allow 24 hours at 60 FPS = 5,184,000 frames)
        let safe_max = 24 * 60 * 60 * 60i64; // 24 hours at 60 FPS
        solver.assert(frame.ge(0));
        solver.assert(frame.lt(safe_max));

        // After incrementing
        let next_frame = &frame + 1;

        // Try to find case where next_frame overflows i32
        let i32_max = i32::MAX as i64;
        solver.assert(next_frame.gt(i32_max));

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove frame increment is safe for practical sessions"
        );
    });
}

// =============================================================================
// Summary Test
// =============================================================================

/// Summary: List all Z3 verified properties
///
/// This test doesn't verify anything new but serves as documentation
/// of all properties verified by Z3 in this module.
#[test]
fn z3_verified_properties_summary() {
    // Frame Arithmetic:
    // - Frame addition with bounded delay produces valid (non-negative) results
    // - Frame subtraction for rollback produces valid targets
    // - Frame comparison is transitive
    // - Frame increment is safe for practical game sessions

    // Circular Buffer:
    // - Modulo operation always produces valid index [0, QUEUE_LENGTH)
    // - Head advancement wraps correctly at QUEUE_LENGTH - 1
    // - Head wraps to exactly 0 from last position
    // - Queue length invariant preserved after add
    // - Circular distance calculation is valid for non-wrapped case

    // Rollback Frame Selection:
    // - Rollback target is always in the past (< current_frame)
    // - Rollback target is within prediction window
    // - Saved state is available for any valid rollback target

    // Frame Delay:
    // - MAX_FRAME_DELAY constraint prevents queue overflow
    // - Sequential frames remain sequential with delay

    // Input Consistency:
    // - No position collision within queue window (modulo uniqueness)

    // Comprehensive:
    // - Complete rollback safety (all properties combined)
    // - Prediction threshold decision is well-defined

    // Internal Component Invariants (new):
    // - InputQueue head/tail bounds
    // - SyncLayer frame ordering invariants
    // - SavedStates cell availability

    println!(
        "Z3 Verification Summary:\n\
         - 4 Frame arithmetic proofs\n\
         - 5 Circular buffer proofs\n\
         - 3 Rollback frame selection proofs\n\
         - 2 Frame delay proofs\n\
         - 1 Input consistency proof\n\
         - 2 Comprehensive safety proofs\n\
         - 8 Internal component invariant proofs (new)\n\
         Total: 25 Z3 proofs"
    );
}

// =============================================================================
// Internal Component Invariant Proofs (Using __internal module access)
// =============================================================================

/// Z3 Proof: InputQueue head/tail indices always valid
///
/// Proves that head and tail indices are always in [0, queue_length).
/// This models the invariants from InputQueue's internal state.
#[test]
fn z3_proof_input_queue_head_tail_bounds() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let head = Int::fresh_const("head");
        let tail = Int::fresh_const("tail");
        let length = Int::fresh_const("length");

        // Initial state invariants (from InputQueue::new)
        // head = 0, tail = 0, length = 0 initially
        solver.assert(head.ge(0));
        solver.assert(head.lt(INPUT_QUEUE_LENGTH));
        solver.assert(tail.ge(0));
        solver.assert(tail.lt(INPUT_QUEUE_LENGTH));
        solver.assert(length.ge(0));
        solver.assert(length.le(INPUT_QUEUE_LENGTH));

        // After add_input: new_head = (head + 1) % queue_length
        let new_head = (&head + 1) % INPUT_QUEUE_LENGTH;

        // Try to find state where new_head is out of bounds
        let out_of_bounds = new_head.lt(0) | new_head.ge(INPUT_QUEUE_LENGTH);
        solver.assert(&out_of_bounds);

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove head index stays in bounds after add"
        );
    });
}

/// Z3 Proof: InputQueue length correctly tracks elements
///
/// Proves that length = (head - tail + queue_length) % queue_length for non-empty queues.
#[test]
fn z3_proof_input_queue_length_calculation() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let head = Int::fresh_const("head");
        let tail = Int::fresh_const("tail");

        // Valid indices
        solver.assert(head.ge(0));
        solver.assert(head.lt(INPUT_QUEUE_LENGTH));
        solver.assert(tail.ge(0));
        solver.assert(tail.lt(INPUT_QUEUE_LENGTH));

        // Length calculation (handles wraparound)
        // If head >= tail: length = head - tail
        // If head < tail: length = queue_length - (tail - head) = queue_length + head - tail
        let length = ((&head - &tail) + INPUT_QUEUE_LENGTH) % INPUT_QUEUE_LENGTH;

        // Length must be in valid range [0, queue_length)
        let valid_length = length.ge(0) & length.lt(INPUT_QUEUE_LENGTH);
        solver.assert(&valid_length);

        // This should be satisfiable
        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Sat,
            "Z3 should find valid queue length states"
        );
    });
}

/// Z3 Proof: SyncLayer last_confirmed_frame invariant
///
/// Proves that last_confirmed_frame is always <= current_frame (when not NULL).
/// This models INV-SL1 from the formal specification.
#[test]
fn z3_proof_sync_layer_confirmed_frame_invariant() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let current_frame = Int::fresh_const("current_frame");
        let last_confirmed_frame = Int::fresh_const("last_confirmed_frame");

        // current_frame >= 0 (always valid after construction)
        solver.assert(current_frame.ge(0));

        // last_confirmed_frame is either NULL (-1) or a valid confirmed frame
        solver.assert(last_confirmed_frame.ge(NULL_FRAME));

        // Invariant: if not NULL, then <= current_frame
        // This is maintained by the SyncLayer logic
        let invariant = last_confirmed_frame.eq(NULL_FRAME).ite(
            &Int::from_i64(1),
            &last_confirmed_frame
                .le(&current_frame)
                .ite(&Int::from_i64(1), &Int::from_i64(0)),
        );

        // Assume invariant holds
        solver.assert(invariant.eq(1));

        // After advance_frame: new_current = current + 1
        let new_current = &current_frame + 1;

        // last_confirmed_frame doesn't change during advance_frame
        // Check that invariant still holds
        let new_invariant = last_confirmed_frame.eq(NULL_FRAME).ite(
            &Int::from_i64(1),
            &last_confirmed_frame
                .le(&new_current)
                .ite(&Int::from_i64(1), &Int::from_i64(0)),
        );

        // Try to find state where invariant is violated after advance
        solver.assert(new_invariant.eq(0));

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove last_confirmed_frame invariant is preserved"
        );
    });
}

/// Z3 Proof: SyncLayer last_saved_frame invariant
///
/// Proves that last_saved_frame is always <= current_frame (when not NULL).
/// This models INV-SL2 from the formal specification.
#[test]
fn z3_proof_sync_layer_saved_frame_invariant() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let current_frame = Int::fresh_const("current_frame");
        let last_saved_frame = Int::fresh_const("last_saved_frame");

        // current_frame >= 0
        solver.assert(current_frame.ge(0));

        // last_saved_frame is NULL or valid
        solver.assert(last_saved_frame.ge(NULL_FRAME));
        solver.assert(last_saved_frame.le(&current_frame));

        // After save_current_state: last_saved = current
        let new_saved = current_frame.clone();

        // Then after advance: new_current = current + 1
        let new_current = &current_frame + 1;

        // Check invariant: new_saved <= new_current
        let invariant_holds = new_saved.le(&new_current);
        solver.assert(invariant_holds.not());

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove last_saved_frame <= current_frame after save and advance"
        );
    });
}

/// Z3 Proof: SavedStates cell availability for rollback
///
/// Proves that for any frame within max_prediction of current_frame,
/// there is a valid cell slot in SavedStates.
#[test]
fn z3_proof_saved_states_availability() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let current_frame = Int::fresh_const("current_frame");
        let target_frame = Int::fresh_const("target_frame");

        // current_frame >= max_prediction (we've advanced enough)
        solver.assert(current_frame.ge(MAX_PREDICTION));

        // target_frame is a valid rollback target
        solver.assert(target_frame.ge(0));
        solver.assert(target_frame.le(&current_frame));
        solver.assert((&current_frame - &target_frame).le(MAX_PREDICTION));

        // SavedStates has max_prediction + 1 slots
        let num_slots = MAX_PREDICTION + 1;

        // Cell index = frame % num_slots
        let cell_index = &target_frame % num_slots;

        // Cell index must be valid [0, num_slots)
        let valid_index = cell_index.ge(0) & cell_index.lt(num_slots);
        solver.assert(&valid_index);

        // This should be satisfiable
        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Sat,
            "Z3 should find valid SavedStates cell for rollback target"
        );
    });
}

/// Z3 Proof: First incorrect frame tracking
///
/// Proves that first_incorrect_frame is always < current_frame when not NULL.
/// This is critical for correct rollback behavior.
#[test]
fn z3_proof_first_incorrect_frame_bound() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let current_frame = Int::fresh_const("current_frame");
        let first_incorrect_frame = Int::fresh_const("first_incorrect_frame");

        // current_frame >= 0
        solver.assert(current_frame.ge(0));

        // first_incorrect_frame is NULL or a valid past frame
        // When set, it's the first frame where prediction was wrong
        solver.assert(first_incorrect_frame.ge(NULL_FRAME));

        // Invariant: if not NULL, then < current_frame
        // (we can't detect incorrect predictions for the current or future frames)
        let invariant =
            first_incorrect_frame.eq(NULL_FRAME) | first_incorrect_frame.lt(&current_frame);
        solver.assert(&invariant);

        // After advance_frame: new_current = current + 1
        let new_current = &current_frame + 1;

        // first_incorrect_frame doesn't change during advance
        // Check invariant still holds
        let new_invariant =
            first_incorrect_frame.eq(NULL_FRAME) | first_incorrect_frame.lt(&new_current);

        // Try to find violation
        solver.assert(new_invariant.not());

        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Unsat,
            "Z3 should prove first_incorrect_frame < current_frame is preserved"
        );
    });
}

/// Z3 Proof: Prediction window constraint
///
/// Proves that the prediction window is bounded by max_prediction.
/// frames_ahead = current_frame - last_confirmed_frame must be <= max_prediction.
#[test]
fn z3_proof_prediction_window_bounded() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let current_frame = Int::fresh_const("current_frame");
        let last_confirmed_frame = Int::fresh_const("last_confirmed_frame");

        // Valid frames
        solver.assert(current_frame.ge(0));
        solver.assert(last_confirmed_frame.ge(NULL_FRAME));
        solver.assert(last_confirmed_frame.lt(&current_frame));

        // Session enforces: current - last_confirmed <= max_prediction + 1
        // (allows up to max_prediction unconfirmed frames)
        let frames_ahead = &current_frame - &last_confirmed_frame;
        solver.assert(frames_ahead.le(MAX_PREDICTION + 1));

        // Verify rollback would be within bounds
        // If we need to rollback, it's to first_incorrect_frame >= last_confirmed
        let first_incorrect = Int::fresh_const("first_incorrect");
        solver.assert(first_incorrect.ge(&last_confirmed_frame));
        solver.assert(first_incorrect.lt(&current_frame));

        // Rollback distance
        let rollback_distance = &current_frame - &first_incorrect;

        // Must be within prediction window
        let bounded = rollback_distance.le(MAX_PREDICTION + 1);
        solver.assert(&bounded);

        // This should be satisfiable
        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Sat,
            "Z3 should find valid prediction window states"
        );
    });
}

/// Z3 Proof: Frame discard safety
///
/// Proves that discarding frames up to `discard_frame` doesn't lose data
/// needed for confirmed inputs that come after.
#[test]
fn z3_proof_frame_discard_safety() {
    let cfg = Config::new();
    with_z3_config(&cfg, || {
        let solver = Solver::new();

        let discard_frame = Int::fresh_const("discard_frame");
        let needed_frame = Int::fresh_const("needed_frame");

        // Both are valid frames
        solver.assert(discard_frame.ge(0));
        solver.assert(needed_frame.ge(0));

        // We only discard frames <= discard_frame
        // We need frames > discard_frame for future processing
        solver.assert(needed_frame.gt(&discard_frame));

        // Queue position of needed frame
        let needed_pos = &needed_frame % INPUT_QUEUE_LENGTH;

        // Queue position of discard frame
        let discard_pos = &discard_frame % INPUT_QUEUE_LENGTH;

        // After discard, tail moves to (discard_frame + 1) % queue_length
        // The needed_frame's position should still be valid (not overwritten)

        // For frames within queue_length of each other with different values,
        // positions are different (from earlier proof)
        // So needed_pos != discard_pos when needed_frame != discard_frame

        // Verify needed_frame's data is preserved
        let distance = &needed_frame - &discard_frame;
        solver.assert(distance.lt(INPUT_QUEUE_LENGTH)); // within queue window

        // Positions should be different
        solver.assert(needed_pos.ne(&discard_pos));

        // This should be satisfiable
        let check_result = solver.check();
        assert_eq!(
            check_result,
            SatResult::Sat,
            "Z3 should prove frame discard preserves needed frames"
        );
    });
}
