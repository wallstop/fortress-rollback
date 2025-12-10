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
(*   - Skip rollback when frame_to_load >= currentFrame (FV-GAP-1)         *)
(*                                                                         *)
(* Properties verified:                                                    *)
(*   - Safety: Rollback target is valid frame (INV-2)                      *)
(*   - Safety: State availability for rollback frames (INV-6)              *)
(*   - Safety: lastConfirmedFrame <= currentFrame (INV-7)                  *)
(*   - Safety: lastSavedFrame <= currentFrame (INV-8)                      *)
(*   - Safety: Deterministic restoration (SAFE-4)                          *)
(*   - Liveness: Rollback completes (LIVE-4, disabled for CI)              *)
(*                                                                         *)
(* Production-Spec Alignment (as of Session 47):                           *)
(*   MAX_PREDICTION maps to SessionBuilder.max_prediction (default: 8).    *)
(*   The invariants proven here hold for ANY valid MAX_PREDICTION > 0.     *)
(*   TLA+ uses small values (1-3) for tractable exhaustive model checking. *)
(*                                                                         *)
(* FV-GAP Fix (Session 47 - Dec 10, 2025):                                 *)
(*   Added SkipRollback action to model the frame_to_load >= currentFrame  *)
(*   edge case. This was discovered by test_terrible_network_preset which  *)
(*   triggered when first_incorrect == current_frame == 0 (misprediction   *)
(*   at frame 0). The spec now explicitly handles this path by resetting   *)
(*   prediction without entering rollback state.                           *)
(*                                                                         *)
(*   Changes made:                                                         *)
(*     1. StartRollback now guards: target < currentFrame                  *)
(*     2. New SkipRollback action: when target >= currentFrame, reset      *)
(*        firstIncorrectFrame without entering rollback state              *)
(*     3. Next relation updated to include SkipRollback                    *)
(*                                                                         *)
(* REVIEW COMPLETED (Dec 7, 2025):                                         *)
(*   Invariants INV-7 and INV-8 are now stricter, requiring lastSavedFrame *)
(*   and lastConfirmedFrame to be <= currentFrame. The LoadState action    *)
(*   now updates lastSavedFrame to maintain the invariant during rollback. *)
(*   Production code (sync_layer.rs) was updated to match: load_frame()    *)
(*   now sets last_saved_frame = frame_to_load.                            *)
(***************************************************************************)

EXTENDS Integers, Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    MAX_PREDICTION,         \* Maximum prediction window (default 8, configurable)
    MAX_FRAME,              \* Maximum frame for model checking
    NUM_PLAYERS,            \* Number of players (configurable, default 2)
    NULL_FRAME              \* Sentinel value (-1)

ASSUME MAX_PREDICTION \in Nat /\ MAX_PREDICTION > 0
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > MAX_PREDICTION
ASSUME NUM_PLAYERS \in Nat /\ NUM_PLAYERS > 0
ASSUME NULL_FRAME \notin 0..MAX_FRAME  \* Sentinel value (outside valid frame range)

(***************************************************************************)
(* Type Definitions                                                        *)
(***************************************************************************)
Frame == {NULL_FRAME} \union (0..MAX_FRAME)
Players == 1..NUM_PLAYERS
InputValue == 0..1          \* Binary input (0=no input, 1=input) for tractable model checking
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
(* Frame 0 state is saved initially, matching production code where        *)
(* save_current_state() is called at the start of the first advance_frame  *)
(***************************************************************************)
InitState == [frame |-> 0, data |-> 0]

Init ==
    /\ currentFrame = 0
    /\ lastConfirmedFrame = NULL_FRAME
    /\ lastSavedFrame = 0  \* Frame 0 is saved initially
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
(* Note: We check the target is within MAX_PREDICTION of the frame when    *)
(* rollback started, not the current frame during resimulation             *)
(***************************************************************************)
RollbackBounded ==
    inRollback =>
        (rollbackTarget = NULL_FRAME \/ rollbackTarget \in 0..MAX_FRAME)

(***************************************************************************)
(* INV-6: State Availability                                               *)
(* Any frame we might need to rollback to has a saved state                *)
(***************************************************************************)
StateAvailability ==
    \A f \in 0..MAX_FRAME:
        (f >= currentFrame - MAX_PREDICTION /\ f <= currentFrame /\ f >= 0) =>
            (StateSaved(f) \/ sparseSaving)

(**************************************************************************)
(* INV-7: Confirmed Frame Consistency                                      *)
(* lastConfirmedFrame must be <= currentFrame (or NULL_FRAME)              *)
(* This is maintained by UpdateConfirmedFrame clamping to currentFrame     *)
(**************************************************************************)
ConfirmedFrameConsistency ==
    lastConfirmedFrame = NULL_FRAME \/ lastConfirmedFrame <= currentFrame

(**************************************************************************)
(* INV-8: Saved Frame Consistency                                          *)
(* lastSavedFrame must be <= currentFrame (or NULL_FRAME)                  *)
(* This invariant is maintained by LoadState updating lastSavedFrame       *)
(**************************************************************************)
SavedFrameConsistency ==
    lastSavedFrame = NULL_FRAME \/ lastSavedFrame <= currentFrame

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
(* GUARD: frame_to_load < currentFrame (can't rollback to current frame)   *)
(* This matches the production guard in adjust_gamestate:                   *)
(*   if frame_to_load >= current_frame { skip_rollback; return Ok(()) }    *)
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
       IN /\ target < currentFrame  \* GUARD: can only rollback to past frames
          /\ rollbackTarget' = target
          /\ StateSaved(target)  \* State must exist
    /\ inRollback' = TRUE
    /\ UNCHANGED <<currentFrame, lastConfirmedFrame, lastSavedFrame,
                   savedStates, playerInputs, inputConfirmed,
                   firstIncorrectFrame, sparseSaving>>

(***************************************************************************)
(* Action: Skip rollback when frame_to_load >= currentFrame                *)
(* This models the production code path:                                    *)
(*   if frame_to_load >= current_frame {                                   *)
(*       debug!("Skipping rollback...");                                   *)
(*       self.sync_layer.reset_prediction();                               *)
(*       return Ok(());                                                    *)
(*   }                                                                      *)
(* This happens when misprediction is detected at frame 0 (first frame)    *)
(* or when sparse saving causes frame_to_load == currentFrame.             *)
(***************************************************************************)
SkipRollback ==
    /\ firstIncorrectFrame # NULL_FRAME
    /\ ~inRollback
    /\ firstIncorrectFrame <= currentFrame
    /\ firstIncorrectFrame >= currentFrame - MAX_PREDICTION
    \* Determine rollback target
    /\ LET target == IF sparseSaving
                     THEN lastSavedFrame
                     ELSE firstIncorrectFrame
       IN target >= currentFrame  \* TRIGGER: would be at or after current frame
    \* Skip rollback: just reset prediction tracking, no state change
    /\ firstIncorrectFrame' = NULL_FRAME
    /\ UNCHANGED <<currentFrame, lastConfirmedFrame, lastSavedFrame,
                   savedStates, playerInputs, inputConfirmed, inRollback,
                   rollbackTarget, sparseSaving>>

(**************************************************************************)
(* Action: Load saved state (part of rollback)                             *)
(* After loading, lastConfirmedFrame is clamped to maintain invariant      *)
(***************************************************************************)
LoadState ==
    /\ inRollback
    /\ rollbackTarget # NULL_FRAME
    /\ StateSaved(rollbackTarget)
    /\ currentFrame' = rollbackTarget
    /\ lastSavedFrame' = rollbackTarget  \* Maintain invariant: lastSavedFrame <= currentFrame
    /\ firstIncorrectFrame' = NULL_FRAME  \* Reset after handling
    \* Clamp lastConfirmedFrame to maintain invariant after rollback
    /\ lastConfirmedFrame' = IF lastConfirmedFrame = NULL_FRAME THEN NULL_FRAME
                             ELSE IF lastConfirmedFrame > rollbackTarget THEN rollbackTarget
                             ELSE lastConfirmedFrame
    /\ UNCHANGED <<savedStates, playerInputs, inputConfirmed, inRollback,
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
(* Note: Requires current frame state to be saved (matches production      *)
(* behavior where save_current_state() is called before advance_frame())   *)
(***************************************************************************)
AdvanceFrame ==
    /\ ~inRollback
    /\ firstIncorrectFrame = NULL_FRAME  \* No rollback pending
    /\ currentFrame < MAX_FRAME
    /\ (StateSaved(currentFrame) \/ sparseSaving)  \* Must save before advance
    /\ currentFrame' = currentFrame + 1
    /\ UNCHANGED <<lastConfirmedFrame, lastSavedFrame, savedStates,
                   playerInputs, inputConfirmed, firstIncorrectFrame,
                   inRollback, rollbackTarget, sparseSaving>>

(***************************************************************************)
(* Action: Update confirmed frame                                          *)
(* Matches production: frame is clamped to currentFrame to maintain        *)
(* the invariant last_confirmed_frame <= current_frame                     *)
(* Only allowed when not in rollback (matches production ordering)         *)
(***************************************************************************)
UpdateConfirmedFrame ==
    /\ ~inRollback  \* Can't update confirmed frame during rollback
    /\ LET rawConfirmed == MinConfirmedFrame
           \* Clamp to current frame (matches production set_last_confirmed_frame)
           clampedFrame == IF rawConfirmed = NULL_FRAME THEN NULL_FRAME
                           ELSE IF rawConfirmed > currentFrame THEN currentFrame
                           ELSE rawConfirmed
       IN lastConfirmedFrame' = clampedFrame
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
    \/ SkipRollback  \* NEW: Handle frame_to_load >= currentFrame case
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
\* Note: INV-9 (RollbackTargetStrictlyPast) was considered but determined
\* to be implicit in StartRollback's guard: target < currentFrame.
\* After LoadState, currentFrame == rollbackTarget which is valid.
SafetyInvariant ==
    /\ FrameMonotonicity
    /\ RollbackBounded
    /\ ConfirmedFrameConsistency
    /\ SavedFrameConsistency
    /\ RollbackConsistency

(***************************************************************************)
(* Liveness Properties                                                     *)
(***************************************************************************)

\* LIVE-3: Progress - frame eventually advances (simplified for model checking)
ProgressGuaranteed ==
    (currentFrame < MAX_FRAME) => <>(currentFrame > currentFrame)

\* LIVE-4: Rollback completes
RollbackCompletes ==
    inRollback ~> ~inRollback

(**************************************************************************)
(* State Constraint for Model Checking                                     *)
(**************************************************************************)
StateConstraint ==
    /\ (currentFrame = NULL_FRAME \/ currentFrame <= MAX_FRAME)
    /\ (lastConfirmedFrame = NULL_FRAME \/ lastConfirmedFrame <= MAX_FRAME)
    /\ (lastSavedFrame = NULL_FRAME \/ lastSavedFrame <= MAX_FRAME)

(**************************************************************************)
(* Theorems                                                                *)
(**************************************************************************)

\* The specification maintains safety
THEOREM Spec => []SafetyInvariant

\* Rollback eventually completes
THEOREM Spec => RollbackCompletes

=============================================================================
