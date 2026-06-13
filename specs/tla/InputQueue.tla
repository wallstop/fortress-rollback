------------------------------ MODULE InputQueue ------------------------------
(***************************************************************************)
(* TLA+ Specification for Fortress Rollback Input Queue                    *)
(*                                                                         *)
(* This module specifies the circular buffer input queue used to store     *)
(* player inputs in Fortress Rollback. It models:                          *)
(*   - Circular buffer operations (add, get, discard)                      *)
(*   - Prediction vs confirmation lifecycle                                *)
(*   - Frame delay handling                                                *)
(*   - First incorrect frame tracking for rollback                         *)
(*   - Graceful-peer-drop freeze: freeze_at / set_frozen_value_at /         *)
(*     roll_confirmed_input_to (the heart of the c25fc1f fix), modeling the  *)
(*     re-roll of the frozen value DOWN to a lowered agreed freeze frame.    *)
(*                                                                         *)
(* Properties verified:                                                    *)
(*   - Safety: No buffer overflow (INV-4)                                  *)
(*   - Safety: Valid indices (INV-5)                                       *)
(*   - Safety: FIFO ordering preserved                                     *)
(*   - Safety: No frame gaps                                               *)
(*   - Safety: Frozen-value determinism (the frozen value is a              *)
(*     deterministic function of the agreed freeze frame and the ring,       *)
(*     independent of which survivor froze or in what order)               *)
(*   - Liveness: Predictions eventually confirmed                          *)
(*                                                                         *)
(* The single-queue freeze actions here are lifted to the cross-survivor    *)
(* agreement level (multiple survivors converging a dropped slot's frozen    *)
(* value to one global-min frame) in the companion FreezeConvergence.tla.   *)
(*                                                                         *)
(* Production-Spec Alignment (as of Phase 9/10):                           *)
(*   QUEUE_LENGTH maps to InputQueueConfig.queue_length (configurable):    *)
(*     - Default: 128 frames (~2.1s at 60 FPS)                             *)
(*     - High latency: 256 frames (~4.3s at 60 FPS)                        *)
(*     - Minimal: 32 frames (~0.5s at 60 FPS)                              *)
(*   The invariants proven here hold for ANY valid QUEUE_LENGTH >= 2.      *)
(*   TLA+ uses small values (3) for tractable exhaustive model checking.   *)
(*                                                                         *)
(* IMPORTANT - Prediction Model (Fortress vs GGPO):                        *)
(*   Production code uses `last_confirmed_input` for prediction source.    *)
(*   This is DIFFERENT from original GGPO which used "last added input".   *)
(*   The `lastConfirmedInput` variable in this spec models production's    *)
(*   `last_confirmed_input` field, which is:                               *)
(*     - Updated when ANY input is added (local or remote)                 *)
(*     - Synchronized across all peers via the network protocol            *)
(*     - Used as the basis for predictions (RepeatLastConfirmed strategy)  *)
(*   This ensures determinism because confirmed inputs are synchronized.   *)
(***************************************************************************)

EXTENDS Integers, Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    QUEUE_LENGTH,           \* Size of circular buffer (configurable: 32-256, default 128)
    MAX_FRAME,              \* Maximum frame number for model checking
    NULL_FRAME              \* Sentinel value for null frame (-1 in impl)

ASSUME QUEUE_LENGTH \in Nat /\ QUEUE_LENGTH > 0
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME NULL_FRAME \notin 0..MAX_FRAME  \* Sentinel value (outside valid frame range)

(***************************************************************************)
(* Variables                                                               *)
(*                                                                         *)
(* NOTE: `lastConfirmedInput` maps to production's `last_confirmed_input`  *)
(* field in InputQueue. It stores the most recent confirmed input value,   *)
(* used as the source for predictions via RepeatLastConfirmed strategy.    *)
(* This is DIFFERENT from original GGPO which used a separate `prediction` *)
(* variable that could desync between peers.                               *)
(***************************************************************************)
VARIABLES
    inputs,                 \* inputs[i] = input at buffer index i
    head,                   \* Next write position
    tail,                   \* Oldest valid input position
    length,                 \* Number of valid entries
    lastAddedFrame,         \* Frame of most recently added input
    lastRequestedFrame,     \* Frame most recently requested (for discard protection)
    firstIncorrectFrame,    \* First frame where prediction was wrong
    lastConfirmedInput,     \* Last confirmed input (used for predictions)
    frameDelay,             \* Runtime input delay in frames
    frozen,                 \* TRUE after graceful peer drop freezes the queue
    freezeFrame             \* The agreed freeze frame this queue is frozen at
                            \* (NULL_FRAME when not frozen, or when frozen via the
                            \* bare `freeze()` with no agreed frame). See FreezeAt.

vars == <<inputs, head, tail, length, lastAddedFrame, lastRequestedFrame,
          firstIncorrectFrame, lastConfirmedInput, frameDelay, frozen,
          freezeFrame>>

(***************************************************************************)
(* Type Definitions                                                        *)
(***************************************************************************)
Frame == {NULL_FRAME} \union (0..MAX_FRAME)
InputValue == 0..3          \* Simplified input representation for model checking
BufferIndex == 0..(QUEUE_LENGTH - 1)

Input == [frame: Frame, value: InputValue]

(***************************************************************************)
(* Type Invariant                                                          *)
(***************************************************************************)
TypeInvariant ==
    /\ inputs \in [BufferIndex -> Input]
    /\ head \in BufferIndex
    /\ tail \in BufferIndex
    /\ length \in 0..QUEUE_LENGTH
    /\ lastAddedFrame \in Frame
    /\ lastRequestedFrame \in Frame
    /\ firstIncorrectFrame \in Frame
    /\ lastConfirmedInput \in Input
    /\ frameDelay \in 0..(QUEUE_LENGTH - 1)
    /\ frozen \in BOOLEAN
    /\ freezeFrame \in Frame

(***************************************************************************)
(* Initial State                                                           *)
(***************************************************************************)
BlankInput == [frame |-> NULL_FRAME, value |-> 0]

Init ==
    /\ inputs = [i \in BufferIndex |-> BlankInput]
    /\ head = 0
    /\ tail = 0
    /\ length = 0
    /\ lastAddedFrame = NULL_FRAME
    /\ lastRequestedFrame = NULL_FRAME
    /\ firstIncorrectFrame = NULL_FRAME
    /\ lastConfirmedInput = BlankInput
    /\ frameDelay = 0
    /\ frozen = FALSE
    /\ freezeFrame = NULL_FRAME

(***************************************************************************)
(* Helper: Modular arithmetic for circular buffer                          *)
(***************************************************************************)
Mod(n) == n % QUEUE_LENGTH

PrevIndex(i) == IF i = 0 THEN QUEUE_LENGTH - 1 ELSE i - 1

NextIndex(i) == Mod(i + 1)

(***************************************************************************)
(* Helper: confirmed-input lookup (models InputQueue::confirmed_input)      *)
(*                                                                         *)
(* Production reads `inputs[frame % queue_length]` and returns it ONLY if   *)
(* the stored entry's frame equals the requested frame (an exact-match ring *)
(* probe; tail/length are not consulted). A non-NULL frame whose ring slot  *)
(* holds a different frame (evicted by wraparound, or never written) has no *)
(* confirmed input. NULL_FRAME never has a confirmed input.                 *)
(***************************************************************************)
HasConfirmedInputAt(f) ==
    /\ f # NULL_FRAME
    /\ inputs[Mod(f)].frame = f

ConfirmedInputAt(f) == inputs[Mod(f)]

(***************************************************************************)
(* INV-4: Queue Length Bounds                                              *)
(* The queue length is always within [0, QUEUE_LENGTH]                     *)
(***************************************************************************)
QueueLengthBounded ==
    length >= 0 /\ length <= QUEUE_LENGTH

(***************************************************************************)
(* INV-5: Queue Index Validity                                             *)
(* Head and tail are always valid buffer indices                           *)
(***************************************************************************)
QueueIndexValid ==
    /\ head \in BufferIndex
    /\ tail \in BufferIndex

(***************************************************************************)
(* Invariant: Length matches head/tail relationship                        *)
(***************************************************************************)
LengthConsistent ==
    IF length = 0
    THEN TRUE  \* Empty queue, head/tail can be anywhere
    ELSE
        IF head >= tail
        THEN length = head - tail
        ELSE length = QUEUE_LENGTH - tail + head

(***************************************************************************)
(* Invariant: No gaps in frame sequence                                    *)
(***************************************************************************)
NoFrameGaps ==
    length > 1 =>
        \A i \in 0..(length - 2):
            LET idx1 == Mod(tail + i)
                idx2 == Mod(tail + i + 1)
            IN inputs[idx1].frame # NULL_FRAME /\ inputs[idx2].frame # NULL_FRAME =>
               inputs[idx2].frame = inputs[idx1].frame + 1

(***************************************************************************)
(* Invariant: FIFO ordering - frames are in ascending order                *)
(***************************************************************************)
FIFOOrdering ==
    length > 1 =>
        \A i \in 0..(length - 2):
            LET idx1 == Mod(tail + i)
                idx2 == Mod(tail + i + 1)
            IN inputs[idx1].frame # NULL_FRAME /\ inputs[idx2].frame # NULL_FRAME =>
               inputs[idx1].frame < inputs[idx2].frame

(***************************************************************************)
(* Action: Add input to queue                                              *)
(* Pre: input.frame = lastAddedFrame + 1 (or lastAddedFrame = NULL_FRAME)  *)
(* Post: Input added, head advanced, length incremented (up to max)        *)
(*                                                                         *)
(* Maps to production: InputQueue::add_input_by_frame()                    *)
(* Updates lastConfirmedInput (production: last_confirmed_input) which is  *)
(* used as the basis for future predictions via RepeatLastConfirmed.       *)
(***************************************************************************)
AddInput(input) ==
    /\ ~frozen
    /\ input.frame \in 0..MAX_FRAME
    /\ input.frame + frameDelay <= MAX_FRAME
    /\ \/ lastAddedFrame = NULL_FRAME
       \/ input.frame + frameDelay = lastAddedFrame + 1
    /\ LET stored == [frame |-> input.frame + frameDelay, value |-> input.value]
       IN /\ inputs' = [inputs EXCEPT ![head] = stored]
          /\ lastConfirmedInput' = stored
    /\ head' = NextIndex(head)
    /\ length' = IF length < QUEUE_LENGTH THEN length + 1 ELSE length
    /\ tail' = IF length >= QUEUE_LENGTH THEN NextIndex(tail) ELSE tail
    /\ lastAddedFrame' = input.frame + frameDelay
    /\ UNCHANGED <<lastRequestedFrame, firstIncorrectFrame, frameDelay, frozen,
                   freezeFrame>>

(***************************************************************************)
(* Action: Get input for a frame (returns confirmed or predicted)          *)
(* This is a read operation - doesn't modify queue state                   *)
(* Post: lastRequestedFrame updated to protect from discard                *)
(***************************************************************************)
GetInput(frame) ==
    /\ frame \in 0..MAX_FRAME
    /\ lastRequestedFrame' = IF frame > lastRequestedFrame
                             THEN frame
                             ELSE lastRequestedFrame
    /\ UNCHANGED <<inputs, head, tail, length, lastAddedFrame,
                   firstIncorrectFrame, lastConfirmedInput, frameDelay, frozen,
                   freezeFrame>>

(***************************************************************************)
(* Action: Add remote input (may detect incorrect prediction)              *)
(* If the input differs from lastConfirmedInput and firstIncorrectFrame    *)
(* is null, set firstIncorrectFrame to this frame.                         *)
(*                                                                         *)
(* Maps to production: InputQueue::add_input_by_frame() when called with   *)
(* remote inputs. The comparison against lastConfirmedInput.value models   *)
(* production's `self.prediction.equal(&input, true)` check, where         *)
(* `self.prediction` was generated from `last_confirmed_input`.            *)
(***************************************************************************)
AddRemoteInput(input) ==
    /\ ~frozen
    /\ input.frame \in 0..MAX_FRAME
    /\ input.frame + frameDelay <= MAX_FRAME
    /\ \/ lastAddedFrame = NULL_FRAME
       \/ input.frame + frameDelay = lastAddedFrame + 1
    /\ LET stored == [frame |-> input.frame + frameDelay, value |-> input.value]
       IN /\ inputs' = [inputs EXCEPT ![head] = stored]
          /\ lastConfirmedInput' = stored
    /\ head' = NextIndex(head)
    /\ length' = IF length < QUEUE_LENGTH THEN length + 1 ELSE length
    /\ tail' = IF length >= QUEUE_LENGTH THEN NextIndex(tail) ELSE tail
    /\ lastAddedFrame' = input.frame + frameDelay
    \* Check if prediction was wrong (compares against last confirmed input)
    /\ IF firstIncorrectFrame = NULL_FRAME /\ input.value # lastConfirmedInput.value
       THEN firstIncorrectFrame' = input.frame + frameDelay
       ELSE firstIncorrectFrame' = firstIncorrectFrame
    /\ UNCHANGED <<lastRequestedFrame, frameDelay, frozen, freezeFrame>>

(***************************************************************************)
(* Action: Discard confirmed inputs up to a frame                          *)
(* Only discards inputs that are before lastRequestedFrame                 *)
(***************************************************************************)
DiscardConfirmed(upToFrame) ==
    /\ upToFrame \in 0..MAX_FRAME
    /\ upToFrame < lastRequestedFrame  \* Protect requested frames
    /\ length > 0
    /\ inputs[tail].frame <= upToFrame
    /\ tail' = NextIndex(tail)
    /\ length' = length - 1
    /\ UNCHANGED <<inputs, head, lastAddedFrame, lastRequestedFrame,
                   firstIncorrectFrame, lastConfirmedInput, frameDelay, frozen,
                   freezeFrame>>

(***************************************************************************)
(* Action: Reset prediction tracking (after rollback)                      *)
(***************************************************************************)
ResetPrediction ==
    /\ firstIncorrectFrame # NULL_FRAME
    /\ firstIncorrectFrame' = NULL_FRAME
    /\ UNCHANGED <<inputs, head, tail, length, lastAddedFrame,
                   lastRequestedFrame, lastConfirmedInput, frameDelay, frozen,
                   freezeFrame>>

(***************************************************************************)
(* Action: Set frame delay                                                  *)
(*                                                                         *)
(* Initial setup may choose any valid delay. Mid-session decreases are      *)
(* rejected as no-ops. Mid-session increases are modeled one frame at a     *)
(* time, filling the new gap with the most recent confirmed input so frame  *)
(* sequence remains contiguous. Repeating this action models larger         *)
(* production delta increases.                                              *)
(***************************************************************************)
SetFrameDelay(newDelay) ==
    /\ newDelay \in 0..(QUEUE_LENGTH - 1)
    /\ \/ /\ newDelay = frameDelay
          /\ UNCHANGED vars
       \/ /\ frozen
          /\ UNCHANGED vars
       \/ /\ lastAddedFrame = NULL_FRAME
          /\ frameDelay' = newDelay
          /\ UNCHANGED <<inputs, head, tail, length, lastAddedFrame,
                         lastRequestedFrame, firstIncorrectFrame,
                         lastConfirmedInput, frozen, freezeFrame>>
       \/ /\ ~frozen
          /\ lastAddedFrame # NULL_FRAME
          /\ newDelay = frameDelay + 1
          /\ lastAddedFrame + 1 <= MAX_FRAME
          /\ length < QUEUE_LENGTH
          /\ LET stored == [frame |-> lastAddedFrame + 1,
                            value |-> lastConfirmedInput.value]
             IN /\ inputs' = [inputs EXCEPT ![head] = stored]
                /\ lastConfirmedInput' = stored
          /\ head' = NextIndex(head)
          /\ length' = length + 1
          /\ tail' = tail
          /\ lastAddedFrame' = lastAddedFrame + 1
          /\ frameDelay' = newDelay
          /\ UNCHANGED <<lastRequestedFrame, firstIncorrectFrame, frozen,
                         freezeFrame>>
       \/ /\ ~frozen
          /\ lastAddedFrame # NULL_FRAME
          /\ newDelay > frameDelay
          /\ length = QUEUE_LENGTH
          /\ UNCHANGED vars
       \/ /\ lastAddedFrame # NULL_FRAME
          /\ newDelay < frameDelay
          /\ UNCHANGED vars

(***************************************************************************)
(* Action: Freeze queue after graceful peer drop (bare `freeze()`)          *)
(*                                                                         *)
(* The bare `freeze()` only flips the flag, preserving the current          *)
(* `lastConfirmedInput` as the frozen value. It makes NO agreed-frame claim *)
(* (freezeFrame stays NULL_FRAME), so FrozenValueDeterminism below does not  *)
(* constrain it. AddInput/AddRemoteInput are disabled while frozen.          *)
(***************************************************************************)
Freeze ==
    /\ ~frozen
    /\ frozen' = TRUE
    /\ freezeFrame' = NULL_FRAME
    /\ UNCHANGED <<inputs, head, tail, length, lastAddedFrame,
                   lastRequestedFrame, firstIncorrectFrame,
                   lastConfirmedInput, frameDelay>>

(***************************************************************************)
(* Action: Freeze the queue AT a specific agreed freeze frame               *)
(*                                                                         *)
(* Maps to production: InputQueue::freeze_at(freeze_frame). This is the      *)
(* graceful-peer-drop entry that ROLLS the frozen value back to the value    *)
(* confirmed at the agreed freeze frame F before freezing, so every survivor *)
(* repeats the SAME deterministic value for the dropped slot. The agreed F   *)
(* is the global minimum across all peers of the dropped slot's received     *)
(* frame (computed in p2p_session::update_player_disconnects), so every       *)
(* survivor has a confirmed input AT F and that value is identical.          *)
(*                                                                         *)
(* Fail-safe (mirrors roll_confirmed_input_to):                             *)
(*   - Idempotent: a no-op once already frozen (never re-seeds a value).     *)
(*   - NULL_FRAME: the expected "no agreed frame yet" case (reserved hot-join *)
(*     slot; drop before any confirmed input) -> freeze silently, value       *)
(*     unchanged, freezeFrame stays NULL (no agreed-frame claim).            *)
(*   - non-NULL present in ring -> roll lastConfirmedInput to that value,     *)
(*     record freezeFrame = f.                                              *)
(*   - non-NULL but missing from ring (evicted / never received) -> value     *)
(*     unchanged (fail-safe; production also logs a Warning), freezeFrame      *)
(*     stays NULL (no value claim is made for a frame we could not roll to).  *)
(***************************************************************************)
FreezeAt(f) ==
    /\ ~frozen
    /\ f \in Frame
    /\ frozen' = TRUE
    /\ IF HasConfirmedInputAt(f)
       THEN /\ lastConfirmedInput' = ConfirmedInputAt(f)
            /\ freezeFrame' = f
       ELSE /\ lastConfirmedInput' = lastConfirmedInput
            /\ freezeFrame' = NULL_FRAME
    /\ UNCHANGED <<inputs, head, tail, length, lastAddedFrame,
                   lastRequestedFrame, firstIncorrectFrame, frameDelay>>

(***************************************************************************)
(* Action: Re-roll an ALREADY-frozen queue's value to frame f               *)
(*                                                                         *)
(* Maps to production: InputQueue::set_frozen_value_at(frame). Where         *)
(* freeze_at seeds the value on the gossip path, the direct-detection paths  *)
(* (own-endpoint timeout, remove_player) freeze at the survivor's OWN, higher *)
(* locally-received frame; the disconnect machinery later converges every     *)
(* survivor's last_frame DOWN to the global-min F and calls this to re-roll   *)
(* the frozen value to track F. The queue-level action rolls to whatever f    *)
(* it is given (monotone-DOWN is a caller/session guarantee, modeled in        *)
(* FreezeConvergence.tla); the value remains a deterministic function of f.   *)
(*                                                                         *)
(* Fail-safe (mirrors set_frozen_value_at):                                 *)
(*   - Not frozen -> no-op (never seeds a value on a live queue).            *)
(*   - NULL_FRAME -> no-op (no agreed frame yet).                            *)
(*   - non-NULL present -> roll lastConfirmedInput, record freezeFrame = f.   *)
(*   - non-NULL missing -> value unchanged (fail-safe; production logs a      *)
(*     Warning), freezeFrame retained at its last successfully-rolled value.  *)
(***************************************************************************)
SetFrozenValueAt(f) ==
    /\ frozen
    /\ f \in Frame
    /\ IF HasConfirmedInputAt(f)
       THEN /\ lastConfirmedInput' = ConfirmedInputAt(f)
            /\ freezeFrame' = f
       ELSE /\ lastConfirmedInput' = lastConfirmedInput
            /\ freezeFrame' = freezeFrame
    /\ UNCHANGED <<inputs, head, tail, length, lastAddedFrame,
                   lastRequestedFrame, firstIncorrectFrame, frameDelay, frozen>>

(***************************************************************************)
(* Next State Relation                                                     *)
(***************************************************************************)
Next ==
    \/ \E v \in InputValue:
        \E f \in 0..MAX_FRAME:
            AddInput([frame |-> f, value |-> v])
    \/ \E v \in InputValue:
        \E f \in 0..MAX_FRAME:
            AddRemoteInput([frame |-> f, value |-> v])
    \/ \E f \in 0..MAX_FRAME: GetInput(f)
    \/ \E f \in 0..MAX_FRAME: DiscardConfirmed(f)
    \/ ResetPrediction
    \/ \E d \in 0..(QUEUE_LENGTH - 1): SetFrameDelay(d)
    \/ Freeze
    \/ \E f \in Frame: FreezeAt(f)
    \/ \E f \in Frame: SetFrozenValueAt(f)

(***************************************************************************)
(* Specification                                                           *)
(***************************************************************************)
Spec == Init /\ [][Next]_vars

(***************************************************************************)
(* Safety Properties (from FORMAL_SPEC.md)                                 *)
(***************************************************************************)

\* SAFE-1: No buffer overflow
NoBufferOverflow ==
    length <= QUEUE_LENGTH

(***************************************************************************)
(* SAFE-FREEZE: Frozen-value determinism (the heart of the c25fc1f fix)     *)
(*                                                                         *)
(* Whenever the queue is frozen at a non-NULL agreed freeze frame F whose    *)
(* confirmed input is present in the ring, `lastConfirmedInput` equals       *)
(* exactly the confirmed input at F. The frozen value is therefore a         *)
(* deterministic function of (F, ring contents at F) -- NOT of which          *)
(* survivor froze, nor of the order in which freeze_at / set_frozen_value_at  *)
(* fired. Combined with the session guarantee that every survivor holds the   *)
(* identical confirmed input at the global-min F, this is precisely what      *)
(* makes survivors repeat byte-identical values for a dropped slot (no        *)
(* desync). FreezeConvergence.tla lifts this to the cross-survivor level.     *)
(*                                                                         *)
(* The guard "HasConfirmedInputAt(freezeFrame)" is load-bearing: a frame      *)
(* later evicted from the ring (DiscardConfirmed then a wrapping write) is     *)
(* the documented F-evicted residual and is deliberately excluded -- the      *)
(* invariant constrains only frames still confirmable, matching production's   *)
(* `confirmed_input` exact-match probe. (While frozen, no wrapping write can   *)
(* occur -- AddInput is disabled -- so a value seeded at freeze stays valid.)  *)
(*                                                                         *)
(* The comparison is on the full Input record. Production's                    *)
(* `last_confirmed_input: Option<T::Input>` stores only the VALUE; this spec    *)
(* additionally carries the `.frame` as bookkeeping (a spec-only ghost, no      *)
(* production field), so this invariant is strictly STRONGER than production's   *)
(* value-only guarantee -- it never weakens it.                                *)
(***************************************************************************)
FrozenValueDeterminism ==
    (frozen /\ freezeFrame # NULL_FRAME /\ HasConfirmedInputAt(freezeFrame))
        => lastConfirmedInput = ConfirmedInputAt(freezeFrame)

(***************************************************************************)
(* SAFE-FREEZE-2: A recorded agreed freeze frame is never NULL while unfrozen *)
(* and, when non-NULL, is a real frame the queue committed to (an agreed       *)
(* frame is only ever recorded by a successful roll). This keeps freezeFrame   *)
(* honest: it is NULL exactly when no agreed-frame value claim is in force.    *)
(***************************************************************************)
FreezeFrameHonest ==
    /\ (~frozen) => (freezeFrame = NULL_FRAME)
    /\ (freezeFrame # NULL_FRAME) => HasConfirmedInputAt(freezeFrame)

\* Combined safety invariant
SafetyInvariant ==
    /\ TypeInvariant
    /\ QueueLengthBounded
    /\ QueueIndexValid
    /\ NoBufferOverflow
    /\ FIFOOrdering
    /\ FrozenValueDeterminism
    /\ FreezeFrameHonest

(***************************************************************************)
(* Liveness Properties                                                     *)
(***************************************************************************)

\* LIVE-2: First incorrect frame eventually cleared (by rollback)
\* Note: This requires fairness assumption on ResetPrediction
IncorrectEventuallyCleared ==
    firstIncorrectFrame # NULL_FRAME ~> firstIncorrectFrame = NULL_FRAME

(**************************************************************************)
(* State Constraint for Model Checking                                     *)
(**************************************************************************)
StateConstraint ==
    /\ (lastAddedFrame = NULL_FRAME \/ lastAddedFrame <= MAX_FRAME)
    /\ length <= QUEUE_LENGTH
    /\ frameDelay <= QUEUE_LENGTH - 1

(**************************************************************************)
(* Theorems                                                                *)
(**************************************************************************)

\* The specification maintains the safety invariant
THEOREM Spec => []SafetyInvariant

=============================================================================
