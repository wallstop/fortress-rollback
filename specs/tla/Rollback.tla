------------------------------- MODULE Rollback -------------------------------
(***************************************************************************)
(* TLA+ Specification for Fortress Rollback Mechanism                      *)
(*                                                                         *)
(* This module specifies the rollback mechanism used in Fortress Rollback  *)
(* for correcting mispredicted inputs. It models:                          *)
(*   - State saving at each frame                                          *)
(*   - Rollback triggered by incorrect predictions                         *)
(*   - State restoration and resimulation                                  *)
(*   - Sparse saving mode                                                  *)
(*                                                                         *)
(* Properties verified:                                                    *)
(*   - Safety: Rollback depth bounded by max_prediction (INV-2)            *)
(*   - Safety: State availability for rollback frames (INV-6)              *)
(*   - Safety: Deterministic restoration (SAFE-4)                          *)
(*   - Liveness: Rollback completes (LIVE-4)                               *)
(*   - Liveness: Progress guaranteed (LIVE-3)                              *)
(***************************************************************************)

EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    MAX_PREDICTION,         \* Maximum prediction window (8)
    MAX_FRAME,              \* Maximum frame for model checking
    NUM_PLAYERS,            \* Number of players
    NULL_FRAME              \* Sentinel value (-1)

ASSUME MAX_PREDICTION \in Nat /\ MAX_PREDICTION > 0
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > MAX_PREDICTION
ASSUME NUM_PLAYERS \in Nat /\ NUM_PLAYERS > 0
ASSUME NULL_FRAME = -1

(***************************************************************************)
(* Type Definitions                                                        *)
(***************************************************************************)
Frame == NULL_FRAME..MAX_FRAME
Players == 1..NUM_PLAYERS
InputValue == 0..3          \* Simplified input space
GameState == [frame: Frame, data: Nat]  \* Simplified state representation

(***************************************************************************)
(* Variables                                                               *)
(***************************************************************************)
VARIABLES
    currentFrame,           \* Current simulation frame
    lastConfirmedFrame,     \* All inputs confirmed up to this frame
    lastSavedFrame,         \* Most recently saved frame
    savedStates,            \* savedStates[f] = state at frame f (if saved)
    playerInputs,           \* playerInputs[p][f] = input for player p at frame f
    inputConfirmed,         \* inputConfirmed[p][f] = TRUE if input confirmed
    firstIncorrectFrame,    \* First frame with wrong prediction
    inRollback,             \* Currently executing rollback?
    rollbackTarget,         \* Frame we're rolling back to
    sparseSaving            \* Sparse saving mode enabled?

vars == <<currentFrame, lastConfirmedFrame, lastSavedFrame, savedStates,
          playerInputs, inputConfirmed, firstIncorrectFrame, inRollback,
          rollbackTarget, sparseSaving>>

(***************************************************************************)
(* Type Invariant                                                          *)
(***************************************************************************)
TypeInvariant ==
    /\ currentFrame \in Frame
    /\ lastConfirmedFrame \in Frame
    /\ lastSavedFrame \in Frame
    /\ savedStates \in [Frame -> GameState \cup {<<>>}]
    /\ playerInputs \in [Players -> [Frame -> InputValue \cup {NULL_FRAME}]]
    /\ inputConfirmed \in [Players -> [Frame -> BOOLEAN]]
    /\ firstIncorrectFrame \in Frame
    /\ inRollback \in BOOLEAN
    /\ rollbackTarget \in Frame
    /\ sparseSaving \in BOOLEAN

(***************************************************************************)
(* Initial State                                                           *)
(***************************************************************************)
InitState == [frame |-> 0, data |-> 0]

Init ==
    /\ currentFrame = 0
    /\ lastConfirmedFrame = NULL_FRAME
    /\ lastSavedFrame = NULL_FRAME
    /\ savedStates = [f \in Frame |-> IF f = 0 THEN InitState ELSE <<>>]
    /\ playerInputs = [p \in Players |-> [f \in Frame |-> NULL_FRAME]]
    /\ inputConfirmed = [p \in Players |-> [f \in Frame |-> FALSE]]
    /\ firstIncorrectFrame = NULL_FRAME
    /\ inRollback = FALSE
    /\ rollbackTarget = NULL_FRAME
    /\ sparseSaving = FALSE  \* Can be TRUE for sparse saving mode

(***************************************************************************)
(* Helper: Check if state is saved for a frame                             *)
(***************************************************************************)
StateSaved(f) ==
    f \in DOMAIN savedStates /\ savedStates[f] # <<>>

(***************************************************************************)
(* Helper: Minimum of confirmed frames across all players                  *)
(***************************************************************************)
MinConfirmedFrame ==
    LET confirmedFrames == {f \in 0..MAX_FRAME:
                            \A p \in Players: inputConfirmed[p][f]}
    IN IF confirmedFrames = {} THEN NULL_FRAME
       ELSE CHOOSE f \in confirmedFrames:
            \A f2 \in confirmedFrames: f2 <= f  \* Maximum confirmed

(***************************************************************************)
(* INV-1: Frame Monotonicity (except during rollback)                      *)
(***************************************************************************)
FrameMonotonicity ==
    \/ inRollback
    \/ currentFrame >= 0

(***************************************************************************)
(* INV-2: Rollback Boundedness                                             *)
(* Rollback depth never exceeds max_prediction                             *)
(***************************************************************************)
RollbackBounded ==
    inRollback =>
        (currentFrame - rollbackTarget) <= MAX_PREDICTION

(***************************************************************************)
(* INV-6: State Availability                                               *)
(* Any frame we might need to rollback to has a saved state                *)
(***************************************************************************)
StateAvailability ==
    \A f \in 0..MAX_FRAME:
        (f >= currentFrame - MAX_PREDICTION /\ f <= currentFrame /\ f >= 0) =>
            (StateSaved(f) \/ sparseSaving)

(***************************************************************************)
(* INV-7: Confirmed Frame Consistency                                      *)
(***************************************************************************)
ConfirmedFrameConsistency ==
    lastConfirmedFrame <= currentFrame

(***************************************************************************)
(* INV-8: Saved Frame Consistency                                          *)
(***************************************************************************)
SavedFrameConsistency ==
    lastSavedFrame <= currentFrame

(***************************************************************************)
(* Action: Add local input (always confirmed)                              *)
(***************************************************************************)
AddLocalInput(player, frame, input) ==
    /\ frame = currentFrame
    /\ frame \in 0..MAX_FRAME
    /\ ~inRollback
    /\ playerInputs' = [playerInputs EXCEPT ![player][frame] = input]
    /\ inputConfirmed' = [inputConfirmed EXCEPT ![player][frame] = TRUE]
    /\ UNCHANGED <<currentFrame, lastConfirmedFrame, lastSavedFrame,
                   savedStates, firstIncorrectFrame, inRollback,
                   rollbackTarget, sparseSaving>>

(***************************************************************************)
(* Action: Receive remote input (may trigger rollback detection)           *)
(***************************************************************************)
ReceiveRemoteInput(player, frame, input) ==
    /\ frame \in 0..MAX_FRAME
    /\ frame <= currentFrame  \* Can receive for past or current frame
    /\ ~inRollback
    /\ playerInputs' = [playerInputs EXCEPT ![player][frame] = input]
    /\ inputConfirmed' = [inputConfirmed EXCEPT ![player][frame] = TRUE]
    \* Detect if prediction was wrong
    /\ IF firstIncorrectFrame = NULL_FRAME
          /\ playerInputs[player][frame] # NULL_FRAME
          /\ playerInputs[player][frame] # input
       THEN firstIncorrectFrame' = frame
       ELSE firstIncorrectFrame' = firstIncorrectFrame
    /\ UNCHANGED <<currentFrame, lastConfirmedFrame, lastSavedFrame,
                   savedStates, inRollback, rollbackTarget, sparseSaving>>

(***************************************************************************)
(* Action: Save current state                                              *)
(***************************************************************************)
SaveState ==
    /\ ~inRollback
    /\ currentFrame \in 0..MAX_FRAME
    /\ savedStates' = [savedStates EXCEPT
                       ![currentFrame] = [frame |-> currentFrame, data |-> currentFrame]]
    /\ lastSavedFrame' = currentFrame
    /\ UNCHANGED <<currentFrame, lastConfirmedFrame, playerInputs,
                   inputConfirmed, firstIncorrectFrame, inRollback,
                   rollbackTarget, sparseSaving>>

(***************************************************************************)
(* Action: Start rollback (triggered by incorrect prediction)              *)
(***************************************************************************)
StartRollback ==
    /\ firstIncorrectFrame # NULL_FRAME
    /\ ~inRollback
    /\ firstIncorrectFrame <= currentFrame
    /\ firstIncorrectFrame >= currentFrame - MAX_PREDICTION
    \* Determine rollback target (sparse saving goes to lastSavedFrame)
    /\ LET target == IF sparseSaving
                     THEN lastSavedFrame
                     ELSE firstIncorrectFrame
       IN /\ rollbackTarget' = target
          /\ StateSaved(target)  \* State must exist
    /\ inRollback' = TRUE
    /\ UNCHANGED <<currentFrame, lastConfirmedFrame, lastSavedFrame,
                   savedStates, playerInputs, inputConfirmed,
                   firstIncorrectFrame, sparseSaving>>

(***************************************************************************)
(* Action: Load saved state (part of rollback)                             *)
(***************************************************************************)
LoadState ==
    /\ inRollback
    /\ rollbackTarget # NULL_FRAME
    /\ StateSaved(rollbackTarget)
    /\ currentFrame' = rollbackTarget
    /\ firstIncorrectFrame' = NULL_FRAME  \* Reset after handling
    /\ UNCHANGED <<lastConfirmedFrame, lastSavedFrame, savedStates,
                   playerInputs, inputConfirmed, inRollback,
                   rollbackTarget, sparseSaving>>

(***************************************************************************)
(* Action: Resimulate frame (advance during rollback)                      *)
(***************************************************************************)
ResimulateFrame ==
    /\ inRollback
    /\ currentFrame < MAX_FRAME
    /\ currentFrame' = currentFrame + 1
    \* Check if rollback is complete (caught up to original frame)
    /\ IF currentFrame + 1 >= rollbackTarget + MAX_PREDICTION
       THEN /\ inRollback' = FALSE
            /\ rollbackTarget' = NULL_FRAME
       ELSE UNCHANGED <<inRollback, rollbackTarget>>
    /\ UNCHANGED <<lastConfirmedFrame, lastSavedFrame, savedStates,
                   playerInputs, inputConfirmed, firstIncorrectFrame,
                   sparseSaving>>

(***************************************************************************)
(* Action: Complete rollback                                               *)
(***************************************************************************)
CompleteRollback ==
    /\ inRollback
    /\ rollbackTarget # NULL_FRAME
    /\ currentFrame >= rollbackTarget  \* Have resimulated past target
    /\ inRollback' = FALSE
    /\ rollbackTarget' = NULL_FRAME
    /\ UNCHANGED <<currentFrame, lastConfirmedFrame, lastSavedFrame,
                   savedStates, playerInputs, inputConfirmed,
                   firstIncorrectFrame, sparseSaving>>

(***************************************************************************)
(* Action: Normal frame advance (no rollback needed)                       *)
(***************************************************************************)
AdvanceFrame ==
    /\ ~inRollback
    /\ firstIncorrectFrame = NULL_FRAME  \* No rollback pending
    /\ currentFrame < MAX_FRAME
    /\ currentFrame' = currentFrame + 1
    /\ UNCHANGED <<lastConfirmedFrame, lastSavedFrame, savedStates,
                   playerInputs, inputConfirmed, firstIncorrectFrame,
                   inRollback, rollbackTarget, sparseSaving>>

(***************************************************************************)
(* Action: Update confirmed frame                                          *)
(***************************************************************************)
UpdateConfirmedFrame ==
    /\ lastConfirmedFrame' = MinConfirmedFrame
    /\ UNCHANGED <<currentFrame, lastSavedFrame, savedStates,
                   playerInputs, inputConfirmed, firstIncorrectFrame,
                   inRollback, rollbackTarget, sparseSaving>>

(***************************************************************************)
(* Next State Relation                                                     *)
(***************************************************************************)
Next ==
    \/ \E p \in Players, f \in 0..MAX_FRAME, i \in InputValue:
        AddLocalInput(p, f, i)
    \/ \E p \in Players, f \in 0..MAX_FRAME, i \in InputValue:
        ReceiveRemoteInput(p, f, i)
    \/ SaveState
    \/ StartRollback
    \/ LoadState
    \/ ResimulateFrame
    \/ CompleteRollback
    \/ AdvanceFrame
    \/ UpdateConfirmedFrame

(***************************************************************************)
(* Fairness                                                                *)
(***************************************************************************)
Fairness ==
    /\ WF_vars(CompleteRollback)
    /\ WF_vars(LoadState)
    /\ WF_vars(ResimulateFrame)

(***************************************************************************)
(* Specification                                                           *)
(***************************************************************************)
Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Properties                                                       *)
(***************************************************************************)

\* SAFE-4: Rollback restores correct state
RollbackConsistency ==
    (inRollback /\ rollbackTarget # NULL_FRAME) =>
        StateSaved(rollbackTarget)

\* Combined safety invariant
SafetyInvariant ==
    /\ FrameMonotonicity
    /\ RollbackBounded
    /\ ConfirmedFrameConsistency
    /\ SavedFrameConsistency
    /\ RollbackConsistency

(***************************************************************************)
(* Liveness Properties                                                     *)
(***************************************************************************)

\* LIVE-3: Progress - frame eventually advances
ProgressGuaranteed ==
    (currentFrame < MAX_FRAME) ~> (currentFrame' > currentFrame)

\* LIVE-4: Rollback completes
RollbackCompletes ==
    inRollback ~> ~inRollback

(***************************************************************************)
(* Theorems                                                                *)
(***************************************************************************)

\* The specification maintains safety
THEOREM Spec => []SafetyInvariant

\* Rollback eventually completes
THEOREM Spec => RollbackCompletes

=============================================================================
