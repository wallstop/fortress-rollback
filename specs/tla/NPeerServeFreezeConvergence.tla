------------------- MODULE NPeerServeFreezeConvergence -------------------
(***************************************************************************)
(* TLA+ model of the N-peer hot-join SERVE GATE against per-survivor       *)
(* freeze-frame convergence (the Session-33 round-5 review Finding 1,       *)
(* coordinator sibling: `npeer_owed_freeze_readjust_at_or_below`).          *)
(*                                                                         *)
(* WHY THIS SPEC EXISTS (de-vacuizing NPeerReactivation's round-5 gates).   *)
(*                                                                         *)
(* `NPeerReactivation.tla` models the activation-frame agreement of an      *)
(* N-peer rejoin, but its committed-history abstraction assumes the         *)
(* pre-attempt per-survivor freeze frame `f0` is UNIFORM across survivors   *)
(* (each survivor's frozen value lines up by construction). Under that      *)
(* assumption the Session-33 round-5 freeze-CONVERGENCE serve gates         *)
(* (acceptance / serve-open deferral; the owed-re-adjust defer/abort) are   *)
(* checked VACUOUSLY: no survivor ever owes a downward re-adjust, so the     *)
(* gate never fires. The NPeerReactivation cfg header records this as the   *)
(* tracked spec-budget residual (session-33 residual 10): "With per-survivor*)
(* f0 and the convergence re-adjust modeled, Agreement over `val` would     *)
(* express exactly that divergence."                                        *)
(*                                                                         *)
(* This companion spec models exactly that: per-PEER freeze frames `f0`     *)
(* that initially DIFFER (asymmetric packet loss → different received-      *)
(* through frames for one dropped slot D), the downward convergence         *)
(* re-adjust each over-frozen peer owes, and the coordinator's serve gate   *)
(* that must defer the snapshot capture while a re-adjust at or below the   *)
(* snapshot frame S is still owed-but-unapplied. A new joiner J' is served  *)
(* a snapshot at S that BAKES the coordinator's view of D's stream for      *)
(* frames 0..S; J' cannot re-adjust below its baseline S (the fold-below-S  *)
(* structural impossibility, S58/S60), so a snapshot captured before the    *)
(* coordinator's own re-adjust is applied is permanently divergent from the *)
(* converged mesh. The gate closes that.                                    *)
(*                                                                         *)
(* It is the companion to NPeerReactivation.tla in exactly the sense        *)
(* SpectatorReactivationEpoch.tla (S56) is the companion to                 *)
(* SpectatorFailover.tla and FreezeConvergence.tla (S37) is the companion   *)
(* to InputQueue.tla: a dedicated spec for a distinct concern an in-place   *)
(* bump would only check vacuously (and, here, would explode the already-   *)
(* at-CI-budget NPeerReactivation state space). It reuses                   *)
(* FreezeConvergence.tla's per-survivor freeze idioms (per-peer received-   *)
(* through, the global-min agreed frame, monotone-down re-roll) and adds    *)
(* the serve-gate interaction those do not model.                          *)
(*                                                                         *)
(* THE GATE (faithful to src/sessions/p2p_session.rs).                      *)
(*                                                                         *)
(* `npeer_owed_freeze_readjust_at_or_below(S)` (p2p_session.rs:1977) folds, *)
(* for a remote dropped slot, over the running non-reserved REMOTE          *)
(* endpoints' claims:                                                       *)
(*     queue_min_confirmed = min over remotes of claim.last_frame           *)
(*     readjust_owed = local_connected \/ status.last_frame > queue_min     *)
(*     fire iff readjust_owed /\ queue_min + 1 <= S                         *)
(* and the serve poll (`poll_npeer_host_serve`, p2p_session.rs:2134) defers *)
(* the capture (pre-capture) or aborts the serve (post-capture) while it    *)
(* fires. The capture itself bakes the coordinator's own connect-status     *)
(* table and confirmed inputs at S (`capture_npeer_snapshot`,               *)
(* p2p_session.rs:2034). Here the dropped slot D is always reported          *)
(* disconnected (it was dropped), so `local_connected` is FALSE and the     *)
(* live arm is `status.last_frame > queue_min` — the per-survivor-f0        *)
(* MINE-DOWN the residual is about. (The `local_connected` first-freeze-    *)
(* propagation arm is the orthogonal not-yet-frozen case, out of this       *)
(* convergence-scope model.)                                                *)
(*                                                                         *)
(* THE FIX_MODE LADDER (the repo's standard negative→positive demo, cf.     *)
(* DoubleFailureRelay / SpectatorReactivationEpoch).                        *)
(*   - FIX_MODE = "GateBlind" — serve as soon as the wait-then-capture      *)
(*     `confirmed >= S` precondition holds, IGNORING the owed re-adjust     *)
(*     (the pre-Finding-1 behavior). DEMO-FAIL: the coordinator bakes its   *)
(*     stale (over-frozen) view of D into the joiner's snapshot, which then *)
(*     permanently diverges from the converged mesh (NoServeDivergence      *)
(*     VIOLATED). This is the model-level RED that proves the gate is       *)
(*     load-bearing.                                                        *)
(*   - FIX_MODE = "Gate" — serve only when no re-adjust at or below S is    *)
(*     owed (the landed gate). PASS: every served snapshot already reflects *)
(*     the converged value for frames <= S, so the joiner agrees with the   *)
(*     mesh forever.                                                        *)
(*                                                                         *)
(* SCOPE (honest).                                                          *)
(*   - The agreed freeze frame for D in the gossip is the true GlobalMin    *)
(*     (every survivor's gossiped claim is its current freeze, delivered    *)
(*     instantly): the Finding-1 hazard is precisely that the gossip min    *)
(*     is seen INSTANTLY by the fold while the coordinator's OWN re-adjust  *)
(*     (the `update_player_disconnects` mine-down + re-roll, which runs     *)
(*     only inside `advance_frame`) LAGS. Gossip LOSS/REORDER and the       *)
(*     freeze-BARRIER that holds `confirmed_frame()` at the gossip min      *)
(*     until convergence (S32/S55) are orthogonal and modeled elsewhere     *)
(*     (DoubleFailureRelay.tla, FreezeConvergence.tla); the barrier is what *)
(*     guarantees a serve is contemplated only once the gossip has          *)
(*     converged, which this spec assumes by construction.                  *)
(*   - One dropped slot D, one returning joiner J'. Multiple survivors      *)
(*     exercise the fold's `min`.                                           *)
(*   - `receivedThrough \in 0..MAX_FRAME` (>= 0): D is dropped after         *)
(*     delivering at least frame 0, so every peer has a confirmed input at   *)
(*     the global min. The production `NULL_FRAME (-1)` freeze (a slot       *)
(*     frozen with NO confirmed input) is excluded — a faithful scoping for  *)
(*     the dropped-after-frame-0 convergence this spec targets (NULL_FRAME   *)
(*     is reserved here only for the not-yet-served joinerFreeze sentinel).  *)
(***************************************************************************)

EXTENDS Naturals, FiniteSets, TLC

CONSTANTS
    SURVIVORS,      \* the REMOTE survivors the coordinator folds over (>= 1)
    COORD,          \* the serving coordinator (holds local_connect_status)
    MAX_FRAME,      \* highest frame number (small bound)
    NULL_FRAME,     \* sentinel for "joiner not yet served" (-1 in impl)
    FIX_MODE        \* "GateBlind" (demo-FAIL) | "Gate" (the landed gate)

ASSUME SURVIVORS # {}
ASSUME COORD \notin SURVIVORS
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME NULL_FRAME \notin 0..MAX_FRAME
ASSUME FIX_MODE \in {"GateBlind", "Gate"}

Peers == SURVIVORS \union {COORD}

(***************************************************************************)
(* The dropped peer's input alphabet. Two distinct values are enough to     *)
(* make divergence observable; kept tiny for tractability.                  *)
(***************************************************************************)
InputValue == 0..1

Frame == {NULL_FRAME} \union (0..MAX_FRAME)

VARIABLES
    streamValue,    \* [0..MAX_FRAME -> InputValue]: D's actual per-frame input
    receivedThrough,\* [Peers -> 0..MAX_FRAME]: per-peer f0 (asymmetric loss)
    localFreeze,    \* [Peers -> 0..MAX_FRAME]: peer's CURRENT freeze for D
                    \*   (its gossiped connect-status last_frame AND the frame
                    \*   its state is rolled to — they converge together in
                    \*   `update_player_disconnects`); starts at receivedThrough,
                    \*   re-adjusts monotone-DOWN toward GlobalMin.
    serveFrame,     \* 0..MAX_FRAME: the snapshot frame S (chosen at Init)
    served,         \* BOOLEAN: the coordinator has served the joiner
    joinerFreeze    \* Frame: the freeze frame BAKED into the joiner's snapshot
                    \*   at serve time (= localFreeze[COORD] then); NULL_FRAME
                    \*   until served; immutable after (J' cannot re-adjust
                    \*   below its snapshot baseline — fold-below-S, S58/S60).

vars == <<streamValue, receivedThrough, localFreeze, serveFrame, served,
          joinerFreeze>>

(***************************************************************************)
(* Min over a non-empty set of frame numbers. CHOOSE ranges over Int frame  *)
(* values only (never over Peers), so it is symmetry-safe.                  *)
(***************************************************************************)
Min(S) == CHOOSE x \in S : \A y \in S : x <= y

(***************************************************************************)
(* The true agreed global-min freeze frame for D: the minimum, across ALL   *)
(* peers, of the received-through high-water. Because frames are contiguous  *)
(* from 0 and every peer received >= GlobalMin, the value at GlobalMin       *)
(* (streamValue[GlobalMin]) is confirmed by every peer and is the value the  *)
(* whole mesh converges to repeat for every frame above GlobalMin.          *)
(***************************************************************************)
GlobalMin == Min({receivedThrough[p] : p \in Peers})

(***************************************************************************)
(* The value peer p currently REPORTS for D at frame g: the real received    *)
(* value at or below its freeze frame, the repeated frozen value above it.   *)
(* (For g <= localFreeze[p] <= receivedThrough[p], p genuinely received g.)   *)
(***************************************************************************)
Report(p, g) ==
    IF g <= localFreeze[p] THEN streamValue[g] ELSE streamValue[localFreeze[p]]

(***************************************************************************)
(* The value EVERY peer eventually reports for D at frame g, once the mesh   *)
(* has converged to GlobalMin (localFreeze[p] = GlobalMin for all p): the    *)
(* real value at/below GlobalMin, the frozen GlobalMin value above it. This  *)
(* is the convergence target the joiner's baked snapshot must match.         *)
(***************************************************************************)
ConvergedValue(g) ==
    IF g <= GlobalMin THEN streamValue[g] ELSE streamValue[GlobalMin]

(***************************************************************************)
(* The joiner's BAKED report for D at frame g (after serve): the frozen      *)
(* stream of the coordinator's snapshot, captured at joinerFreeze and        *)
(* immutable.                                                                *)
(***************************************************************************)
JoinerReport(g) ==
    IF g <= joinerFreeze THEN streamValue[g] ELSE streamValue[joinerFreeze]

(***************************************************************************)
(* The serve gate (faithful to `npeer_owed_freeze_readjust_at_or_below`).    *)
(* D is reported disconnected by every folded survivor, so queue_connected   *)
(* is FALSE (never skipped) and any_folded is TRUE (SURVIVORS # {}). D is    *)
(* locally disconnected too, so local_connected is FALSE and the live arm    *)
(* is the mine-down `status.last_frame > queue_min`.                         *)
(***************************************************************************)
QueueMin == Min({localFreeze[s] : s \in SURVIVORS})

ReadjustOwed == localFreeze[COORD] > QueueMin           \* local_connected = FALSE

OwedAtOrBelow(s) == ReadjustOwed /\ (QueueMin + 1 <= s)

(***************************************************************************)
(* Type invariant.                                                         *)
(***************************************************************************)
TypeInvariant ==
    /\ streamValue \in [0..MAX_FRAME -> InputValue]
    /\ receivedThrough \in [Peers -> 0..MAX_FRAME]
    /\ localFreeze \in [Peers -> 0..MAX_FRAME]
    /\ serveFrame \in 0..MAX_FRAME
    /\ served \in BOOLEAN
    /\ joinerFreeze \in Frame

(***************************************************************************)
(* Init. The stream, the per-peer received-through (the per-survivor f0),    *)
(* and the snapshot frame S are fixed by an adversarial nondeterministic     *)
(* choice. Every peer has already frozen D at its own received-through (the  *)
(* drop happened); none has converged yet; no serve has occurred.            *)
(***************************************************************************)
Init ==
    /\ streamValue \in [0..MAX_FRAME -> InputValue]
    /\ receivedThrough \in [Peers -> 0..MAX_FRAME]
    /\ localFreeze = receivedThrough
    /\ serveFrame \in 0..MAX_FRAME
    /\ served = FALSE
    /\ joinerFreeze = NULL_FRAME

(***************************************************************************)
(* ConvergeDown -- a peer applies its owed downward re-adjust, rolling its    *)
(* freeze for D from its current frame down to a lower agreed frame f, never  *)
(* below the true global min. Maps to production: the                        *)
(* `update_player_disconnects` mine-down (`status.last_frame =                *)
(* min(last_frame, agreed)`) + `set_frozen_value_at` re-roll + forced         *)
(* re-simulation, which run only inside `advance_frame` (hence the lag the    *)
(* gate guards against).                                                      *)
(***************************************************************************)
ConvergeDown(p, f) ==
    /\ f \in GlobalMin..(localFreeze[p] - 1)
    /\ localFreeze' = [localFreeze EXCEPT ![p] = f]
    /\ UNCHANGED <<streamValue, receivedThrough, serveFrame, served,
                   joinerFreeze>>

(***************************************************************************)
(* Serve -- the coordinator captures the snapshot at S and serves the joiner,*)
(* baking its CURRENT view of D (localFreeze[COORD]) into the snapshot. The   *)
(* FIX_MODE selects the gate:                                                 *)
(*   - "GateBlind": serve unconditionally (the wait-then-capture              *)
(*     `confirmed >= S` precondition is assumed met; this mode omits the      *)
(*     Finding-1 owed-re-adjust check).                                       *)
(*   - "Gate": serve only when no re-adjust at or below S is owed.            *)
(***************************************************************************)
GatePasses ==
    CASE FIX_MODE = "GateBlind" -> TRUE
      [] FIX_MODE = "Gate"      -> ~OwedAtOrBelow(serveFrame)

Serve ==
    /\ ~served
    /\ GatePasses
    /\ served' = TRUE
    /\ joinerFreeze' = localFreeze[COORD]
    /\ UNCHANGED <<streamValue, receivedThrough, localFreeze, serveFrame>>

Next ==
    \/ \E p \in Peers, f \in 0..MAX_FRAME: ConvergeDown(p, f)
    \/ Serve

(***************************************************************************)
(* Fairness. Convergence and the (gated) serve are weakly fair so the mesh   *)
(* reaches the converged + served fixpoint; nothing forces a premature        *)
(* serve.                                                                     *)
(***************************************************************************)
Fairness ==
    /\ \A p \in Peers : WF_vars(\E f \in 0..MAX_FRAME : ConvergeDown(p, f))
    /\ WF_vars(Serve)

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety properties.                                                       *)
(***************************************************************************)

(***************************************************************************)
(* Sanity: no peer ever freezes below the agreed global min or above what     *)
(* it actually received (the FreezeConvergence FreezeFrameInRange lift). The  *)
(* lower bound is the desync-relevant half.                                   *)
(***************************************************************************)
FreezeFrameInRange ==
    \A p \in Peers :
        /\ localFreeze[p] >= GlobalMin
        /\ localFreeze[p] <= receivedThrough[p]

(***************************************************************************)
(* HEADLINE -- NoServeDivergence. Once the joiner has been served, its baked   *)
(* report for D agrees, at EVERY frame at or below the snapshot frame S, with  *)
(* the value the whole mesh converges to. The joiner cannot re-adjust below    *)
(* its baseline, so a snapshot baked from a stale (over-frozen) coordinator    *)
(* view is a PERMANENT cross-peer confirmed-state desync — exactly the         *)
(* divergence NPeerReactivation's gates were vacuously assumed to prevent.     *)
(*                                                                            *)
(* Stated against ConvergedValue (the known fixpoint target) rather than a     *)
(* live peer so it is checkable the instant `served` becomes TRUE, not only    *)
(* at the convergence fixpoint: under "GateBlind" TLC reports the violation    *)
(* immediately on a premature serve; under "Gate" it never can.               *)
(***************************************************************************)
NoServeDivergence ==
    served => \A g \in 0..serveFrame : JoinerReport(g) = ConvergedValue(g)

(***************************************************************************)
(* The user-facing cross-peer corollary (mirrors FreezeConvergence's          *)
(* ConvergedNoDesync). Once the joiner is served AND the mesh has converged    *)
(* (every peer's freeze = GlobalMin), the joiner's baked D-stream is            *)
(* byte-identical to EVERY live peer's reported D-stream at every frame <= S.   *)
(* This is the corollary of NoServeDivergence evaluated at the AllConverged     *)
(* fixpoint (where Report(p, g) = ConvergedValue(g)), not an independent guard  *)
(* — but "no cross-peer desync" is the property the whole gate exists to        *)
(* deliver, so it is stated explicitly. EventuallyServed + ConvergeDown         *)
(* fairness prove its AllConverged /\ served hypothesis is reachable (so it is   *)
(* not vacuously true).                                                         *)
(***************************************************************************)
AllConverged == \A p \in Peers : localFreeze[p] = GlobalMin

ServedMeshNoDesync ==
    (served /\ AllConverged) =>
        \A p \in Peers, g \in 0..serveFrame : JoinerReport(g) = Report(p, g)

SafetyInvariant ==
    /\ TypeInvariant
    /\ FreezeFrameInRange
    /\ NoServeDivergence
    /\ ServedMeshNoDesync

(***************************************************************************)
(* Liveness: the protocol always reaches the served fixpoint (the gate never  *)
(* deadlocks the serve — every owed re-adjust is eventually applied, after     *)
(* which the gate opens). Under the weak-fairness assumptions: every           *)
(* over-frozen peer eventually ConvergeDowns to GlobalMin, so ReadjustOwed     *)
(* clears and Serve is enabled.                                                *)
(***************************************************************************)
EventuallyServed == <>served

(***************************************************************************)
(* Non-vacuity witness (see NPeerServeFreezeConvergence_Witness.cfg). The      *)
(* "Gate" PASS is meaningful only if the gate actually has work to do — i.e.   *)
(* a reachable state exists where the serve is NOT yet done AND a re-adjust at  *)
(* or below S is owed, so the gate is actively DEFERRING a serve that          *)
(* "GateBlind" would take (and that would then fail NoServeDivergence). This    *)
(* invariant is deliberately FALSIFIABLE: TLC reports it VIOLATED, exhibiting   *)
(* that exact gate-deferral state, proving the "Gate" coverage is non-vacuous   *)
(* (the gate is not merely satisfied because no serve is ever owed-blocked).    *)
(***************************************************************************)
WitnessGateDefers == ~(~served /\ OwedAtOrBelow(serveFrame))

THEOREM Spec => []SafetyInvariant

=============================================================================
