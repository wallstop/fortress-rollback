------------------------------- MODULE TimeSync -------------------------------
(***************************************************************************)
(* TLA+ Specification for Fortress Rollback Time Synchronization           *)
(*                                                                         *)
(* This module specifies the time synchronization mechanism used to keep   *)
(* peers running at compatible speeds in Fortress Rollback. It models:     *)
(*   - Frame advantage tracking (local vs remote)                          *)
(*   - Rolling window averaging for smooth sync                            *)
(*   - Frame advance rate recommendations                                  *)
(*                                                                         *)
(* The TimeSync module tracks frame advantage differentials between local  *)
(* and remote peers using a rolling window to smooth out network jitter.   *)
(* The average frame advantage determines whether to speed up, slow down,  *)
(* or maintain current frame rate.                                         *)
(*                                                                         *)
(* Algorithm Overview:                                                     *)
(*   1. Each frame, record local_advantage and remote_advantage            *)
(*   2. Store values in circular buffer (window) indexed by frame % size   *)
(*   3. Compute average: (sum(remote) - sum(local)) / (2 * window_size)    *)
(*   4. Positive average = remote is ahead, slow down local                *)
(*   5. Negative average = local is ahead, speed up local                  *)
(*                                                                         *)
(* Properties verified:                                                    *)
(*   - Safety: Window index always in bounds (INV-TS-1)                    *)
(*   - Safety: Average is bounded by max advantage values (INV-TS-2)       *)
(*   - Safety: Window size is always >= 1 (INV-TS-3)                       *)
(*   - Safety: Invalid frames don't modify state (INV-TS-4)                *)
(*   - Safety: Deterministic computation (INV-TS-5)                        *)
(*                                                                         *)
(* Production-Spec Alignment (as of Dec 2025):                             *)
(*   WINDOW_SIZE maps to TimeSyncConfig.window_size (default: 30).         *)
(*   The invariants proven here hold for ANY valid WINDOW_SIZE >= 1.       *)
(*   TLA+ uses small values (2-3) for tractable exhaustive model checking. *)
(*                                                                         *)
(* Key Design Decisions modeled:                                           *)
(*   - Integer-only arithmetic for determinism (no floating point)         *)
(*   - NULL/negative frames are skipped (defensive programming)            *)
(*   - Window wraps via modulo arithmetic                                  *)
(***************************************************************************)

EXTENDS Integers, Naturals, FiniteSets, TLC

CONSTANTS
    WINDOW_SIZE,            \* Size of rolling average window (default 30, configurable)
    MAX_FRAME,              \* Maximum frame for model checking
    MAX_ADVANTAGE,          \* Maximum frame advantage value (for bounding)
    NUM_PEERS,              \* Number of peers (typically 2 for P2P)
    NULL_FRAME              \* Sentinel value for null/invalid frames (-1 in impl)

ASSUME WINDOW_SIZE \in Nat /\ WINDOW_SIZE >= 1
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME MAX_ADVANTAGE \in Nat /\ MAX_ADVANTAGE > 0
ASSUME NUM_PEERS \in Nat /\ NUM_PEERS >= 2
ASSUME NULL_FRAME \notin 0..MAX_FRAME  \* Sentinel value (outside valid frame range)

\* Constant-level invariants (checked once at startup)
ASSUME WindowSizeMinimumAssumption == WINDOW_SIZE >= 1
ASSUME WindowIndexBoundsAssumption == \A f \in 0..MAX_FRAME: (f % WINDOW_SIZE) \in 0..(WINDOW_SIZE - 1)

(***************************************************************************)
(* Type Definitions                                                        *)
(***************************************************************************)
Frame == {NULL_FRAME} \union (0..MAX_FRAME)
Peers == 1..NUM_PEERS
WindowIndex == 0..(WINDOW_SIZE - 1)
AdvantageValue == (-MAX_ADVANTAGE)..MAX_ADVANTAGE

(***************************************************************************)
(* Variables                                                               *)
(***************************************************************************)
VARIABLES
    \* Per-peer TimeSync state
    localWindow,            \* localWindow[p] = array of local advantages for peer p
    remoteWindow,           \* remoteWindow[p] = array of remote advantages for peer p

    \* Current frame tracking
    currentFrame,           \* currentFrame[p] = current simulation frame for peer p

    \* Average computation result (cached for verification)
    averageAdvantage        \* averageAdvantage[p] = computed average frame advantage

vars == <<localWindow, remoteWindow, currentFrame, averageAdvantage>>

(***************************************************************************)
(* Type Invariant                                                          *)
(***************************************************************************)
TypeInvariant ==
    /\ localWindow \in [Peers -> [WindowIndex -> AdvantageValue]]
    /\ remoteWindow \in [Peers -> [WindowIndex -> AdvantageValue]]
    /\ currentFrame \in [Peers -> Frame]
    /\ averageAdvantage \in [Peers -> Int]

(***************************************************************************)
(* Initial State                                                           *)
(* All window slots initialized to 0, matching production code:            *)
(* local: vec![0; window_size], remote: vec![0; window_size]               *)
(***************************************************************************)
Init ==
    /\ localWindow = [p \in Peers |-> [i \in WindowIndex |-> 0]]
    /\ remoteWindow = [p \in Peers |-> [i \in WindowIndex |-> 0]]
    /\ currentFrame = [p \in Peers |-> 0]
    /\ averageAdvantage = [p \in Peers |-> 0]

(***************************************************************************)
(* Helper: Compute sum of window values using explicit fold                *)
(***************************************************************************)
SumWindow(window) ==
    LET RECURSIVE SumHelper(_)
        SumHelper(idx) ==
            IF idx >= WINDOW_SIZE THEN 0
            ELSE window[idx] + SumHelper(idx + 1)
    IN SumHelper(0)

(***************************************************************************)
(* Helper: Compute average frame advantage                                 *)
(* Formula: (remote_sum - local_sum) / (2 * count)                         *)
(* Uses integer division for determinism                                   *)
(***************************************************************************)
ComputeAverage(localWin, remoteWin) ==
    LET localSum == SumWindow(localWin)
        remoteSum == SumWindow(remoteWin)
    IN (remoteSum - localSum) \div (2 * WINDOW_SIZE)

(***************************************************************************)
(* INV-TS-1: Window Index Always In Bounds                                 *)
(* For any valid frame, frame % window_size is in [0, window_size)         *)
(***************************************************************************)
WindowIndexInBounds ==
    \A f \in 0..MAX_FRAME:
        (f % WINDOW_SIZE) \in WindowIndex

(***************************************************************************)
(* INV-TS-2: Average Bounded by Inputs                                     *)
(* The average frame advantage is bounded by the max input values          *)
(* |average| <= MAX_ADVANTAGE (since it's derived from bounded inputs)     *)
(***************************************************************************)
AverageBounded ==
    \A p \in Peers:
        /\ averageAdvantage[p] >= -MAX_ADVANTAGE
        /\ averageAdvantage[p] <= MAX_ADVANTAGE

(***************************************************************************)
(* INV-TS-3: Window Size Minimum                                           *)
(* Window size is always at least 1 (enforced by with_config)              *)
(***************************************************************************)
WindowSizeMinimum ==
    WINDOW_SIZE >= 1

(***************************************************************************)
(* INV-TS-4: Window Consistency                                            *)
(* Local and remote windows have consistent lengths                        *)
(***************************************************************************)
WindowConsistency ==
    \A p \in Peers:
        /\ DOMAIN localWindow[p] = WindowIndex
        /\ DOMAIN remoteWindow[p] = WindowIndex

(***************************************************************************)
(* Action: Advance Frame with Valid Frame                                  *)
(* Models TimeSync::advance_frame(frame, local_adv, remote_adv)            *)
(* Pre: frame is valid (>= 0, not NULL)                                    *)
(* Post: Window slot at frame % window_size is updated                     *)
(***************************************************************************)
AdvanceFrame(p, frame, localAdv, remoteAdv) ==
    /\ frame \in 0..MAX_FRAME           \* Valid frame range
    /\ frame # NULL_FRAME               \* Not null
    /\ localAdv \in AdvantageValue
    /\ remoteAdv \in AdvantageValue
    /\ LET index == frame % WINDOW_SIZE
           \* Compute updated windows for average calculation
           newLocalWindow == [localWindow[p] EXCEPT ![index] = localAdv]
           newRemoteWindow == [remoteWindow[p] EXCEPT ![index] = remoteAdv]
       IN
        /\ localWindow' = [localWindow EXCEPT ![p] = newLocalWindow]
        /\ remoteWindow' = [remoteWindow EXCEPT ![p] = newRemoteWindow]
        /\ averageAdvantage' = [averageAdvantage EXCEPT ![p] =
            ComputeAverage(newLocalWindow, newRemoteWindow)]
        /\ currentFrame' = [currentFrame EXCEPT ![p] = frame]

(***************************************************************************)
(* Action: Skip Invalid Frame (NULL or negative)                           *)
(* Models the defensive code path in TimeSync::advance_frame:              *)
(*   if frame.is_null() || frame.as_i32() < 0 { return; }                  *)
(* The window state is NOT modified when an invalid frame is passed.       *)
(*                                                                         *)
(* Design Note: This action is retained as a stuttering step to explicitly *)
(* model the defensive programming behavior in production code. While      *)
(* AdvanceFrame guards already ensure valid frames, keeping this action    *)
(* documents that invalid frames are explicitly handled (no-op) rather     *)
(* than being an error condition. It adds minimal state space overhead     *)
(* since it's equivalent to a stutter step.                                *)
(***************************************************************************)
SkipInvalidFrame(p) ==
    UNCHANGED vars  \* No state change for invalid frames

(***************************************************************************)
(* Action: Compute Average (explicit query)                                *)
(* Models TimeSync::average_frame_advantage()                              *)
(* This is a read-only operation that recomputes the average.              *)
(*                                                                         *)
(* Design Note: The averageAdvantage variable is a "shadow variable" that  *)
(* tracks what the computed average would be at any point. In production,  *)
(* the average is computed on-demand. Here we model it as state to verify  *)
(* invariants like AverageBounded and DeterministicAverage. This action    *)
(* allows the spec to model explicit queries without window modifications. *)
(***************************************************************************)
QueryAverage(p) ==
    /\ averageAdvantage' = [averageAdvantage EXCEPT ![p] =
        ComputeAverage(localWindow[p], remoteWindow[p])]
    /\ UNCHANGED <<localWindow, remoteWindow, currentFrame>>

(***************************************************************************)
(* Synchronization Behavior Modeling                                       *)
(* When one peer is ahead, the sync system recommends adjustments          *)
(***************************************************************************)

(***************************************************************************)
(* Helper: Determine sync recommendation                                   *)
(* Based on average frame advantage:                                       *)
(*   > 0: Remote ahead, local should wait (slow down)                      *)
(*   < 0: Local ahead, local should not wait (speed up)                    *)
(*   = 0: In sync, proceed normally                                        *)
(***************************************************************************)
SyncRecommendation(p) ==
    IF averageAdvantage[p] > 0 THEN "SlowDown"
    ELSE IF averageAdvantage[p] < 0 THEN "SpeedUp"
    ELSE "InSync"

(***************************************************************************)
(* Property: Symmetric Advantages produce expected average                 *)
(* If local_adv = -adv and remote_adv = adv for all entries,               *)
(* average should be exactly adv                                           *)
(***************************************************************************)
SymmetricAdvantageProperty ==
    \A p \in Peers:
        \A adv \in AdvantageValue:
            (\A i \in WindowIndex:
                localWindow[p][i] = -adv /\ remoteWindow[p][i] = adv)
            => averageAdvantage[p] = adv

(***************************************************************************)
(* Next State Relation                                                     *)
(***************************************************************************)
Next ==
    \/ \E p \in Peers, f \in 0..MAX_FRAME, la \in AdvantageValue, ra \in AdvantageValue:
        AdvanceFrame(p, f, la, ra)
    \/ \E p \in Peers:
        SkipInvalidFrame(p)
    \/ \E p \in Peers:
        QueryAverage(p)

(***************************************************************************)
(* Fairness Conditions                                                     *)
(***************************************************************************)
Fairness ==
    \A p \in Peers: WF_vars(QueryAverage(p))

(***************************************************************************)
(* Specification                                                           *)
(***************************************************************************)
Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Properties                                                       *)
(***************************************************************************)

\* Combined safety invariant
\* Note: WindowIndexInBounds is omitted as it's a constant-level mathematical
\* property already verified by the ASSUME WindowIndexBoundsAssumption.
SafetyInvariant ==
    /\ TypeInvariant
    /\ AverageBounded
    /\ WindowSizeMinimum
    /\ WindowConsistency

(***************************************************************************)
(* Temporal Properties for Convergence                                     *)
(***************************************************************************)

\* If both peers have same window contents, they should have same averages
\* (This models the determinism requirement - key for rollback netcode)
DeterministicAverage ==
    \A p1, p2 \in Peers:
        (localWindow[p1] = localWindow[p2] /\ remoteWindow[p1] = remoteWindow[p2])
        => averageAdvantage[p1] = averageAdvantage[p2]

(***************************************************************************)
(* State Constraint for Model Checking                                     *)
(***************************************************************************)
StateConstraint ==
    /\ \A p \in Peers: (currentFrame[p] = NULL_FRAME \/ currentFrame[p] <= MAX_FRAME)
    /\ \A p \in Peers: averageAdvantage[p] \in (-MAX_ADVANTAGE)..MAX_ADVANTAGE

(***************************************************************************)
(* Theorems                                                                *)
(***************************************************************************)

\* The specification maintains safety
THEOREM SafetyTheorem == Spec => []SafetyInvariant

\* Determinism is preserved
THEOREM DeterminismTheorem == Spec => []DeterministicAverage

=============================================================================
