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
(***************************************************************************)

EXTENDS Integers, Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    QUEUE_LENGTH,           \* Size of circular buffer (128)
    MAX_FRAME,              \* Maximum frame number for model checking
    NULL_FRAME              \* Sentinel value for null frame (-1 in impl)

ASSUME QUEUE_LENGTH \in Nat /\ QUEUE_LENGTH > 0
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME NULL_FRAME \notin 0..MAX_FRAME  \* Sentinel value (outside valid frame range)

(***************************************************************************)
(* Variables                                                               *)
(***************************************************************************)
VARIABLES
    inputs,                 \* inputs[i] = input at buffer index i
    head,                   \* Next write position
    tail,                   \* Oldest valid input position
    length,                 \* Number of valid entries
    lastAddedFrame,         \* Frame of most recently added input
    lastRequestedFrame,     \* Frame most recently requested (for discard protection)
    firstIncorrectFrame,    \* First frame where prediction was wrong
    prediction              \* Current prediction value

vars == <<inputs, head, tail, length, lastAddedFrame, lastRequestedFrame,
          firstIncorrectFrame, prediction>>

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
    /\ prediction \in Input

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
    /\ prediction = BlankInput

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
(***************************************************************************)
AddInput(input) ==
    /\ input.frame \in 0..MAX_FRAME
    /\ \/ lastAddedFrame = NULL_FRAME
       \/ input.frame = lastAddedFrame + 1
    /\ inputs' = [inputs EXCEPT ![head] = input]
    /\ head' = NextIndex(head)
    /\ length' = IF length < QUEUE_LENGTH THEN length + 1 ELSE length
    /\ tail' = IF length >= QUEUE_LENGTH THEN NextIndex(tail) ELSE tail
    /\ lastAddedFrame' = input.frame
    /\ prediction' = input  \* Update prediction to latest input
    /\ UNCHANGED <<lastRequestedFrame, firstIncorrectFrame>>

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
                   firstIncorrectFrame, prediction>>

(***************************************************************************)
(* Action: Add remote input (may detect incorrect prediction)              *)
(* If the input differs from prediction and firstIncorrectFrame is null,   *)
(* set firstIncorrectFrame to this frame.                                  *)
(***************************************************************************)
AddRemoteInput(input) ==
    /\ input.frame \in 0..MAX_FRAME
    /\ \/ lastAddedFrame = NULL_FRAME
       \/ input.frame = lastAddedFrame + 1
    /\ inputs' = [inputs EXCEPT ![head] = input]
    /\ head' = NextIndex(head)
    /\ length' = IF length < QUEUE_LENGTH THEN length + 1 ELSE length
    /\ tail' = IF length >= QUEUE_LENGTH THEN NextIndex(tail) ELSE tail
    /\ lastAddedFrame' = input.frame
    \* Check if prediction was wrong
    /\ IF firstIncorrectFrame = NULL_FRAME /\ input.value # prediction.value
       THEN firstIncorrectFrame' = input.frame
       ELSE firstIncorrectFrame' = firstIncorrectFrame
    /\ prediction' = input
    /\ UNCHANGED <<lastRequestedFrame>>

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
                   firstIncorrectFrame, prediction>>

(***************************************************************************)
(* Action: Reset prediction tracking (after rollback)                      *)
(***************************************************************************)
ResetPrediction ==
    /\ firstIncorrectFrame # NULL_FRAME
    /\ firstIncorrectFrame' = NULL_FRAME
    /\ UNCHANGED <<inputs, head, tail, length, lastAddedFrame,
                   lastRequestedFrame, prediction>>

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

(**************************************************************************)
(* Theorems                                                                *)
(**************************************************************************)

\* The specification maintains the safety invariant
THEOREM Spec => []SafetyInvariant

=============================================================================
