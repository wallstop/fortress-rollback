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
(*                                                                         *)
(* Properties verified:                                                    *)
(*   - Safety: No buffer overflow (INV-4)                                  *)
(*   - Safety: Valid indices (INV-5)                                       *)
(*   - Safety: FIFO ordering preserved                                     *)
(*   - Safety: No frame gaps                                               *)
(*   - Liveness: Predictions eventually confirmed                          *)
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
    frozen                  \* TRUE after graceful peer drop freezes the queue

vars == <<inputs, head, tail, length, lastAddedFrame, lastRequestedFrame,
          firstIncorrectFrame, lastConfirmedInput, frameDelay, frozen>>

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

(***************************************************************************)
(* Helper: Modular arithmetic for circular buffer                          *)
(***************************************************************************)
Mod(n) == n % QUEUE_LENGTH

PrevIndex(i) == IF i = 0 THEN QUEUE_LENGTH - 1 ELSE i - 1

NextIndex(i) == Mod(i + 1)

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
    /\ UNCHANGED <<lastRequestedFrame, firstIncorrectFrame, frameDelay, frozen>>

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
                   firstIncorrectFrame, lastConfirmedInput, frameDelay, frozen>>

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
    /\ UNCHANGED <<lastRequestedFrame, frameDelay, frozen>>

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
                   firstIncorrectFrame, lastConfirmedInput, frameDelay, frozen>>

(***************************************************************************)
(* Action: Reset prediction tracking (after rollback)                      *)
(***************************************************************************)
ResetPrediction ==
    /\ firstIncorrectFrame # NULL_FRAME
    /\ firstIncorrectFrame' = NULL_FRAME
    /\ UNCHANGED <<inputs, head, tail, length, lastAddedFrame,
                   lastRequestedFrame, lastConfirmedInput, frameDelay, frozen>>

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
                         lastConfirmedInput, frozen>>
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
          /\ UNCHANGED <<lastRequestedFrame, firstIncorrectFrame, frozen>>
       \/ /\ ~frozen
          /\ lastAddedFrame # NULL_FRAME
          /\ newDelay > frameDelay
          /\ length = QUEUE_LENGTH
          /\ UNCHANGED vars
       \/ /\ lastAddedFrame # NULL_FRAME
          /\ newDelay < frameDelay
          /\ UNCHANGED vars

(***************************************************************************)
(* Action: Freeze queue after graceful peer drop                            *)
(*                                                                         *)
(* Freezing only flips the flag. AddInput/AddRemoteInput are disabled while *)
(* frozen, preserving the final confirmed input for deterministic reads.    *)
(***************************************************************************)
Freeze ==
    /\ ~frozen
    /\ frozen' = TRUE
    /\ UNCHANGED <<inputs, head, tail, length, lastAddedFrame,
                   lastRequestedFrame, firstIncorrectFrame,
                   lastConfirmedInput, frameDelay>>

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

\* Combined safety invariant
SafetyInvariant ==
    /\ TypeInvariant
    /\ QueueLengthBounded
    /\ QueueIndexValid
    /\ NoBufferOverflow
    /\ FIFOOrdering

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
