--------------------------- MODULE FreezeConvergence ---------------------------
(***************************************************************************)
(* TLA+ Specification for cross-survivor freeze-value convergence          *)
(* (the c25fc1f graceful-peer-drop fix, lifted from one queue to the mesh). *)
(*                                                                         *)
(* WHAT THIS MODELS (and why it is the companion to InputQueue.tla)         *)
(*                                                                         *)
(* When a peer drops in an N>=3 full-mesh session under                     *)
(* DisconnectBehavior::ContinueWithout, every survivor freezes the dropped   *)
(* slot so it repeats the dropped peer's last good input forever. Under      *)
(* asymmetric packet loss, survivors received the dropped peer's inputs      *)
(* through DIFFERENT frames. If each survivor simply repeated its own last    *)
(* received input, the survivors would repeat DIFFERENT values -> a silent    *)
(* confirmed-stream desync (the original bug fixed in commit c25fc1f).        *)
(*                                                                         *)
(* The fix: the session computes one AGREED freeze frame F = the global       *)
(* minimum, across all peers, of the dropped slot's received frame, and       *)
(* every survivor rolls its frozen value to the value confirmed at F (via      *)
(* InputQueue::freeze_at / set_frozen_value_at). Because F is the global min,   *)
(* every survivor has a confirmed input at F and (confirmed inputs being        *)
(* byte-identical across peers) that value is identical -- so every survivor    *)
(* repeats the SAME value. Survivors that detected the drop directly froze at   *)
(* their own higher local frame and converge DOWN to F as the lowering gossip   *)
(* propagates.                                                                 *)
(*                                                                         *)
(* InputQueue.tla proves the SINGLE-queue mechanism (freeze_at rolls the value  *)
(* to a deterministic function of (F, ring)). This module proves the            *)
(* CROSS-SURVIVOR property the audit flagged as unmodeled: once every survivor  *)
(* converges to the global-min F, the dropped slot's reported confirmed stream  *)
(* is byte-identical across all survivors (no desync), and no survivor ever      *)
(* freezes below the global min. The ring buffer is abstracted away (proven      *)
(* in InputQueue.tla); here a survivor's confirmed inputs for the dropped slot   *)
(* are the deterministic per-frame stream it received.                          *)
(*                                                                         *)
(* Properties verified:                                                    *)
(*   - Safety: frozen value is always the stream value at the freeze frame   *)
(*     (FrozenValueFaithful -- the single-queue determinism, per survivor)   *)
(*   - Safety: a frozen survivor's frame is in [GlobalMin, receivedThrough]   *)
(*     (FreezeFrameInRange -- never freezes below the agreed global min)     *)
(*   - Safety: once all survivors converge to GlobalMin, the dropped slot's   *)
(*     reported stream is identical across survivors (ConvergedNoDesync)      *)
(*   - Liveness: every survivor eventually converges to GlobalMin            *)
(*     (EventuallyConverged, under fairness)                                 *)
(***************************************************************************)

EXTENDS Naturals, FiniteSets, TLC

CONSTANTS
    SURVIVORS,              \* Set of surviving peers (e.g. {s1, s2, s3})
    MAX_FRAME,              \* Maximum frame number for model checking
    NULL_FRAME              \* Sentinel for "not yet frozen" (-1 in impl)

ASSUME SURVIVORS # {}
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME NULL_FRAME \notin 0..MAX_FRAME

(***************************************************************************)
(* The dropped peer's input alphabet. Two distinct values are enough to     *)
(* make divergence observable; kept tiny for tractability.                  *)
(***************************************************************************)
InputValue == 0..1

Frame == {NULL_FRAME} \union (0..MAX_FRAME)

(***************************************************************************)
(* Variables                                                               *)
(*                                                                         *)
(* streamValue and receivedThrough are chosen nondeterministically at Init  *)
(* and never change -- they model, respectively, the dropped peer's actual   *)
(* per-frame input (a single deterministic stream; confirmed inputs are       *)
(* byte-identical across peers, so survivors never disagree on the VALUE of   *)
(* a frame they both received) and each survivor's high-water received frame  *)
(* under asymmetric loss (its confirmed inputs are frames 0..receivedThrough, *)
(* contiguous, as the queue guarantees).                                     *)
(***************************************************************************)
VARIABLES
    streamValue,            \* [0..MAX_FRAME -> InputValue]: dropped peer's inputs
    receivedThrough,        \* [SURVIVORS -> 0..MAX_FRAME]: per-survivor high-water
    frozenAt,               \* [SURVIVORS -> Frame]: frame each survivor froze at
    frozenValue             \* [SURVIVORS -> InputValue]: value each survivor repeats

vars == <<streamValue, receivedThrough, frozenAt, frozenValue>>

(***************************************************************************)
(* Min over a non-empty set of frame numbers. The CHOOSE ranges over Int     *)
(* frame values only (never over SURVIVORS), so it is symmetry-safe.         *)
(***************************************************************************)
Min(S) == CHOOSE x \in S : \A y \in S : x <= y

(***************************************************************************)
(* The agreed global-min freeze frame F: the minimum, across all survivors,  *)
(* of the dropped slot's received frame. Because every survivor received      *)
(* through >= GlobalMin (it IS the min) and frames are contiguous from 0,     *)
(* every survivor has a confirmed input at GlobalMin and that value           *)
(* (streamValue[GlobalMin]) is shared.                                        *)
(***************************************************************************)
GlobalMin == Min({receivedThrough[s] : s \in SURVIVORS})

IsFrozen(s) == frozenAt[s] # NULL_FRAME

(***************************************************************************)
(* Type Invariant                                                          *)
(***************************************************************************)
TypeInvariant ==
    /\ streamValue \in [0..MAX_FRAME -> InputValue]
    /\ receivedThrough \in [SURVIVORS -> 0..MAX_FRAME]
    /\ frozenAt \in [SURVIVORS -> Frame]
    /\ frozenValue \in [SURVIVORS -> InputValue]

(***************************************************************************)
(* Initial State                                                           *)
(*                                                                         *)
(* The stream and per-survivor received high-water marks are fixed by an     *)
(* adversarial nondeterministic choice; no survivor has frozen yet.          *)
(***************************************************************************)
Init ==
    /\ streamValue \in [0..MAX_FRAME -> InputValue]
    /\ receivedThrough \in [SURVIVORS -> 0..MAX_FRAME]
    /\ frozenAt = [s \in SURVIVORS |-> NULL_FRAME]
    /\ frozenValue = [s \in SURVIVORS |-> 0]

(***************************************************************************)
(* Action: a survivor freezes the dropped slot at an agreed frame f.        *)
(*                                                                         *)
(* Maps to production: InputQueue::freeze_at(f) at the freeze transition.    *)
(* A survivor can freeze at any frame it has actually received               *)
(* (f <= receivedThrough[s]) and no lower than the true global min           *)
(* (f >= GlobalMin) -- because the agreed frame is a minimum of REAL received *)
(* frames gossiped by peers, no peer ever reports (and the min never reaches) *)
(* a frame below GlobalMin. The two endpoints model the two production paths: *)
(*   - f = receivedThrough[s]: the survivor that detected the drop directly,  *)
(*     freezing at its own (possibly higher) local frame.                     *)
(*   - GlobalMin <= f < receivedThrough[s]: a survivor freezing from a         *)
(*     partially-converged gossip minimum.                                    *)
(* The frozen value is the stream value at f -- the deterministic roll.        *)
(***************************************************************************)
FreezeAtFrame(s, f) ==
    /\ ~IsFrozen(s)
    /\ f \in GlobalMin..receivedThrough[s]
    /\ frozenAt' = [frozenAt EXCEPT ![s] = f]
    /\ frozenValue' = [frozenValue EXCEPT ![s] = streamValue[f]]
    /\ UNCHANGED <<streamValue, receivedThrough>>

(***************************************************************************)
(* Action: an already-frozen survivor rolls its frozen value DOWN to a       *)
(* lower agreed frame f as the lowering gossip arrives.                      *)
(*                                                                         *)
(* Maps to production: InputQueue::set_frozen_value_at(f), driven from        *)
(* disconnect_player_at_frames mining last_frame DOWN to the converging       *)
(* global min. Monotone-down only (f < the current freeze frame), never       *)
(* below the true global min, and re-rolls the value deterministically.       *)
(***************************************************************************)
ConvergeDown(s, f) ==
    /\ IsFrozen(s)
    /\ f \in GlobalMin..(frozenAt[s] - 1)
    /\ frozenAt' = [frozenAt EXCEPT ![s] = f]
    /\ frozenValue' = [frozenValue EXCEPT ![s] = streamValue[f]]
    /\ UNCHANGED <<streamValue, receivedThrough>>

Next ==
    \/ \E s \in SURVIVORS, f \in 0..MAX_FRAME: FreezeAtFrame(s, f)
    \/ \E s \in SURVIVORS, f \in 0..MAX_FRAME: ConvergeDown(s, f)

(***************************************************************************)
(* Specification. Weak fairness on convergence (and on freezing) drives the  *)
(* liveness property: a survivor stuck above the global min can always roll   *)
(* down, and an unfrozen survivor can always freeze.                         *)
(***************************************************************************)
Fairness ==
    /\ \A s \in SURVIVORS : WF_vars(\E f \in 0..MAX_FRAME : FreezeAtFrame(s, f))
    /\ \A s \in SURVIVORS : WF_vars(\E f \in 0..MAX_FRAME : ConvergeDown(s, f))

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety properties                                                       *)
(***************************************************************************)

(***************************************************************************)
(* The repeated (frozen) value is always exactly the stream value at the     *)
(* frame the survivor is frozen at -- the per-survivor lift of InputQueue's    *)
(* FrozenValueDeterminism. The value is a deterministic function of the        *)
(* freeze frame; it is never a stale "last received" value decoupled from F.   *)
(***************************************************************************)
FrozenValueFaithful ==
    \A s \in SURVIVORS :
        IsFrozen(s) => frozenValue[s] = streamValue[frozenAt[s]]

(***************************************************************************)
(* No survivor freezes below the agreed global min or above what it           *)
(* actually received. The lower bound is the desync-relevant half: a freeze    *)
(* below GlobalMin would repeat a value some peer never confirmed.             *)
(***************************************************************************)
FreezeFrameInRange ==
    \A s \in SURVIVORS :
        IsFrozen(s) =>
            /\ frozenAt[s] >= GlobalMin
            /\ frozenAt[s] <= receivedThrough[s]

(***************************************************************************)
(* The dropped slot's reported confirmed input at game frame g, as seen by a  *)
(* frozen survivor s: the real received value for frames at or below its       *)
(* freeze frame, and the repeated frozen value above it. (For g <= frozenAt[s] *)
(* <= receivedThrough[s], s genuinely received g, so it reports streamValue[g].)*)
(***************************************************************************)
Report(s, g) == IF g <= frozenAt[s] THEN streamValue[g] ELSE frozenValue[s]

(***************************************************************************)
(* The headline desync-closing property. Once every survivor has converged to *)
(* the agreed global min F, the dropped slot's reported confirmed stream is    *)
(* byte-identical across ALL survivors at EVERY frame -- there is no desync.    *)
(* (Pre-convergence the streams legitimately differ in the staggered window;   *)
(* the fix guarantees they re-converge, which this captures at the fixpoint.)  *)
(*                                                                         *)
(* NOTE: this is the cross-survivor CONCLUSION at the convergence fixpoint,   *)
(* the corollary of FrozenValueFaithful evaluated when AllConverged holds --   *)
(* not an independent guard catching a bug class the per-survivor invariant    *)
(* misses (a label-only freeze breaks BOTH). It is stated explicitly because   *)
(* "no desync" is the user-facing property the whole c25fc1f fix exists to     *)
(* deliver; EventuallyConverged proves its AllConverged hypothesis reachable    *)
(* (so it is not vacuously true).                                             *)
(***************************************************************************)
AllConverged == \A s \in SURVIVORS : frozenAt[s] = GlobalMin

ConvergedNoDesync ==
    AllConverged =>
        \A s1, s2 \in SURVIVORS, g \in 0..MAX_FRAME :
            Report(s1, g) = Report(s2, g)

SafetyInvariant ==
    /\ TypeInvariant
    /\ FrozenValueFaithful
    /\ FreezeFrameInRange
    /\ ConvergedNoDesync

(***************************************************************************)
(* Liveness: every survivor eventually converges to the global min (so the    *)
(* mesh reaches the no-desync fixpoint). Under the weak-fairness assumptions   *)
(* above: an unfrozen survivor eventually freezes, and any survivor above       *)
(* GlobalMin can always ConvergeDown to GlobalMin, after which it is stable.    *)
(***************************************************************************)
EventuallyConverged == <>AllConverged

(***************************************************************************)
(* Theorems                                                                *)
(***************************************************************************)
THEOREM Spec => []SafetyInvariant

=============================================================================
