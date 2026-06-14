----------------------------- MODULE DoubleFailureRelay -----------------------------
(***************************************************************************)
(* TLA+ arbitration of the "double-failure relay" residual — the single    *)
(* remaining potential-desync item in N-PLAYER-DESYNC-AUDIT.md (arbitrated  *)
(* REAL, fix deferred-with-spec, in Session 41).                            *)
(*                                                                          *)
(* WHAT THIS MODELS (and why it is the companion to FreezeConvergence.tla)   *)
(*                                                                          *)
(* FreezeConvergence.tla proves that ONCE every survivor converges to the    *)
(* global-min agreed freeze frame F, the dropped slot's reported stream is    *)
(* byte-identical (no desync). It assumes the frozen value is always          *)
(* re-rollable (InputQueue::set_frozen_value_at) — it abstracts away the      *)
(* prediction WINDOW and the IRREVERSIBLE DISCARD of confirmed frames below   *)
(* the window floor, and it abstracts away FOLD MEMBERSHIP (which survivors a  *)
(* given peer can still "see").                                              *)
(*                                                                          *)
(* The double-failure relay residual lives in exactly those two abstracted    *)
(* gaps. The S32 "freeze barrier" bounds each survivor's confirmed frame for a *)
(* dropping slot by the mesh-gossip minimum (`remote_slot_confirmed_bound`),  *)
(* and BOTH that bound and the freeze override (`update_player_disconnects`)  *)
(* iterate the IDENTICAL `is_running()` / non-reserved remote-endpoint set.   *)
(* So within one snapshot, bound == applied freeze, and confirmation can      *)
(* never outrun a freeze the mesh later agrees on — UNLESS the low value's     *)
(* source endpoint LEAVES the fold between confirming and freezing            *)
(* (fold-membership asymmetry). That happens when the global-min ORIGIN       *)
(* survivor DIES (its endpoint pruned from both folds) after relaying its low  *)
(* freeze F to a third survivor but before delivering it to us, AND the relay  *)
(* is partition-delayed past our record+discard of the higher frames. We then  *)
(* confirm and DISCARD the dropped slot's real inputs in (F, window_floor],    *)
(* the late relay forces a rollback below the window floor (the S20 clamp      *)
(* keeps us live), and the discarded frames keep our real-input state while    *)
(* the relayer froze at F -> a permanent cross-survivor confirmed-STATE        *)
(* divergence whose CONVERGED INPUTS nonetheless match (so the input-checksum  *)
(* desync detector stays silent).                                            *)
(*                                                                          *)
(* WHAT THIS MODULE ARBITRATES                                              *)
(*                                                                          *)
(* The S41 write-up records three candidate fixes and the project rule "no    *)
(* partial fix shipped — a partial fix regresses liveness." This module turns  *)
(* the prose arbitration into a machine-checked one. The confirmation rule is  *)
(* a CONSTANT `FIX_MODE` with three values, each exercised by its own .cfg:    *)
(*                                                                          *)
(*   - "Baseline"  — current production (prune the dead origin from the fold,  *)
(*                   confirm/discard on the surviving stale-high cache). The   *)
(*                   safety invariant NoConfirmedDivergence is VIOLATED: TLC   *)
(*                   reproduces the residual as a counterexample. (Run via     *)
(*                   DoubleFailureRelay_Baseline.cfg — expected to FAIL; this  *)
(*                   is the model-level RED that mirrors the in-process repro  *)
(*                   p2p_n4_double_failure_relay_dropped_slot_diverges_*.)     *)
(*                                                                          *)
(*   - "Tombstone" — candidate fix #2 (non-wire): keep folding a dead          *)
(*                   survivor's last gossiped term. Safety holds, but the      *)
(*                   liveness property ConfirmationProgresses is VIOLATED: a    *)
(*                   dead survivor that held a stale-low view of a slot that    *)
(*                   is in fact still LIVE pins every survivor's confirmation   *)
(*                   forever (a survivor cannot tell a real freeze from        *)
(*                   ordinary lag at the moment of death — see                 *)
(*                   N-PLAYER-DESYNC-AUDIT.md / the S25 critic-#3 `is_running`  *)
(*                   arbitration). This is the formal proof of "a partial fix  *)
(*                   regresses liveness." (Run via                            *)
(*                   DoubleFailureRelay_Tombstone.cfg — expected to FAIL on    *)
(*                   the PROPERTY, not on safety.)                            *)
(*                                                                          *)
(*   - "MeshAgree" — candidate fix #3 (the sound one): a survivor advances       *)
(*                   confirmation of a not-yet-mesh-agreed slot only to the      *)
(*                   MESH-ACKED FLOOR — the min, over the local view and every    *)
(*                   still-alive peer REACHABLE over a live link, of that peer's  *)
(*                   CURRENT floor (a fresh ack, NOT the possibly-stale          *)
(*                   per-endpoint cache the barrier folds) — and HOLDS entirely  *)
(*                   while any alive peer is unreachable (the ack round cannot    *)
(*                   complete). It therefore never discards a frame a peer has    *)
(*                   already frozen lower or a partitioned peer might lower.      *)
(*                   BOTH the safety invariant AND the liveness property hold.    *)
(*                   This is the default config (DoubleFailureRelay.cfg,         *)
(*                   registered in verify-tla.sh) and the design a future        *)
(*                   production red-green cycle should implement (a per-slot     *)
(*                   ack/epoch on connect-status gossip).                       *)
(*                                                                          *)
(* Properties:                                                              *)
(*   - Safety: NoConfirmedDivergence — no two alive survivors ever hold        *)
(*     divergent recorded confirmed state for the dropped slot at any frame    *)
(*     both have committed. (VIOLATED under Baseline; holds under              *)
(*     Tombstone/MeshAgree.)                                                  *)
(*   - Safety: FreezeNeverBelowGlobalMin / BoundNeverBelowCommit — sanity      *)
(*     invariants on the fold arithmetic.                                     *)
(*   - Liveness: ConfirmationProgresses — every alive survivor eventually      *)
(*     confirms the dropped slot through to its proper target (the mesh        *)
(*     reaches a stable, fully-confirmed fixpoint). (VIOLATED under            *)
(*     Tombstone; holds under Baseline/MeshAgree.)                            *)
(*                                                                          *)
(* FAITHFUL ABSTRACTIONS (honest scope)                                      *)
(*   - The dropped slot D is modeled by a per-survivor high-water RECEIPT      *)
(*     `recvThrough` (its asymmetric-loss confirmed range) rather than a       *)
(*     packet stream; this is the FreezeConvergence convention. The agreed     *)
(*     freeze frame is GlobalMin == Min(recvThrough).                          *)
(*   - Confirmed INPUT bytes are injective in the source frame (realInput[f]   *)
(*     == f, so two survivors recorded the SAME byte at frame g iff they        *)
(*     recorded it from the same SOURCE frame). We therefore track only the    *)
(*     recorded SOURCE frame `recSrc` (g for a real record, the freeze frame   *)
(*     for a frozen record) and compare those — a byte divergence iff the       *)
(*     source frames differ. This drops the stream variable with no loss.      *)
(*   - The prediction window is the constant WINDOW; a committed frame g is     *)
(*     LOCKED (irreversibly discarded, no longer re-rollable) once bound has    *)
(*     advanced past g + WINDOW. This is the S20-clamp / ring-floor mechanism. *)
(*   - A peer "death" (an explicit remove_player or a disconnect timeout) is    *)
(*     modeled by `alive[s] = FALSE`, which prunes s from every other          *)
(*     survivor's fold (the production `!endpoint.is_running()` skip).         *)
(*   - "Partition" between src and obs is `~link[src][obs]`; gossip from src    *)
(*     reaches obs only while the link is up. A down link does NOT prune the    *)
(*     endpoint (production keeps it `Running` until the long timeout) — that   *)
(*     decoupling of "link down" from "endpoint pruned" is exactly the          *)
(*     fold-membership asymmetry the residual exploits. Partitions are present  *)
(*     post-warmup (chosen by Init) and only HEAL (monotone-up links; see       *)
(*     Unblock) — faithful to the repro (warm up all-open, sever, re-open) and  *)
(*     it keeps the liveness obligation a plain `<>[]` (no oscillation to       *)
(*     defend against, so weak fairness and a small TLC tableau suffice).       *)
(*   - The MeshAgree fix is modeled at the policy level ("hold confirmation     *)
(*     of a not-yet-mesh-agreed slot while partitioned from any alive peer")    *)
(*     rather than as a concrete ack-round wire format; the wire format is the  *)
(*     production design choice this proof informs, not constrains.            *)
(*                                                                          *)
(* SCOPE OF THE THREE RESULTS (honest bounds — what each claim does and does NOT *)
(* prove; established in an adversarial faithfulness review):                    *)
(*                                                                          *)
(*   - The NEGATIVE results are UNCONDITIONAL within the modeled world. Baseline  *)
(*     -unsafe and Tombstone-illiveness are demonstrated by REACHABLE             *)
(*     counterexamples, and every abstraction here is conservative (it makes the  *)
(*     model safer/more-restrictive than production), so a reachable violation is *)
(*     a real one. These two are the load-bearing arbitration outcomes.          *)
(*                                                                          *)
(*   - The POSITIVE result (MeshAgree sound) is proven for the AGGREGATION RULE   *)
(*     under two idealizing assumptions that the production fix must additionally *)
(*     discharge — both already tracked as the deferred wire-epoch work (S41 fix  *)
(*     candidate #1, the per-slot drop-epoch on connect-status gossip):           *)
(*       (a) INSTANTANEOUS, FRESH ACK. `MeshAckedFloor` reads each reachable alive *)
(*           peer's CURRENT floor (`PeerFloor`) with no in-flight delay. Real      *)
(*           floors converge monotone-DOWN, so a DELAYED ack reads a stale-HIGH    *)
(*           floor; discarding against it could lock a frame the peer later        *)
(*           freezes below — exactly this residual, recursed one level. The        *)
(*           production fix must therefore make the acked floor a COMMITMENT       *)
(*           ("I will not freeze below X"), which is what the drop-epoch provides;  *)
(*           a bare floor snapshot is not enough. This model proves the floor      *)
(*           AGGREGATION (min over reachable-alive committed floors, hold-on-       *)
(*           partition, exclude-on-convergence) is the right POLICY; the epoch is   *)
(*           what turns a snapshot into the commitment the policy assumes.          *)
(*       (b) SYNCHRONIZED DEATH. `alive[s]` is a single global flag; production     *)
(*           death is per-observer (a peer can be `Running` in one survivor's       *)
(*           registry and `Disconnected` in another's during the disconnect-timeout *)
(*           skew). The fix's ack-set is computed from this shared alive view;       *)
(*           production must tolerate survivors briefly disagreeing on who is alive. *)
(*       Also: this is the NON-hot-join world — the bound fold's hot-join            *)
(*       `attempt_clamp` / reserved-endpoint arms and the merge's                   *)
(*       `disconnect_requested` / reactivation-floor skips are out of scope (a       *)
(*       second instance of this residual may live in the hot-join reactivation      *)
(*       paths, which share the fold-pruning relay shape). And the MeshAgree         *)
(*       LIVENESS pass assumes monotone-healing partitions (no flap); the Tombstone  *)
(*       liveness FAILURE is structural and needs no such assumption.               *)
(***************************************************************************)

EXTENDS Integers, FiniteSets, TLC

CONSTANTS
    SURVIVORS,     \* observing survivors, e.g. {a, b, c} (a=us, b=dying origin, c=relay)
    MAX_FRAME,     \* maximum frame number for model checking
    NULL_FRAME,    \* sentinel "no frame" (-1 in impl)
    WINDOW,        \* prediction window: frames > bound - WINDOW are still re-rollable
    RECEIPTS,      \* allowed per-survivor receipts of the dropped slot (adversarial at Init)
    FIX_MODE       \* "Baseline" | "Tombstone" | "MeshAgree"

ASSUME SURVIVORS # {}
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME NULL_FRAME \notin 0..MAX_FRAME
ASSUME WINDOW \in Nat /\ WINDOW >= 1
ASSUME RECEIPTS \subseteq (0..MAX_FRAME) /\ RECEIPTS # {}
ASSUME FIX_MODE \in {"Baseline", "Tombstone", "MeshAgree"}

Frame == {NULL_FRAME} \union (0..MAX_FRAME)

(***************************************************************************)
(* Variables.                                                              *)
(*                                                                          *)
(* recvThrough is an adversarial fixed-at-Init choice (each survivor's        *)
(* high-water received frame of the dropped slot under asymmetric loss). The  *)
(* rest evolve.                                                              *)
(***************************************************************************)
VARIABLES
    recvThrough,   \* [SURVIVORS -> 0..MAX_FRAME]: per-survivor receipt of dropped slot
    alive,         \* [SURVIVORS -> BOOLEAN]: still running in the mesh (is_running)
    localDisc,     \* [SURVIVORS -> BOOLEAN]: my local_connect_status[D].disconnected
    localFrame,    \* [SURVIVORS -> Frame]: my local_connect_status[D].last_frame (freeze once disc)
    cacheDisc,     \* [SURVIVORS -> [SURVIVORS -> BOOLEAN]]: cacheDisc[obs][src] = obs's cache of src's D-disc
    cacheLast,     \* [SURVIVORS -> [SURVIVORS -> Frame]]: cacheLast[obs][src] = obs's cache of src's D last_frame
    link,          \* [SURVIVORS -> [SURVIVORS -> BOOLEAN]]: link[src][obs] = src->obs gossip flows
    bound,         \* [SURVIVORS -> Frame]: confirmed high-water for D (NULL = nothing confirmed)
    recSrc         \* [SURVIVORS -> [0..MAX_FRAME -> Frame]]: recorded source frame per committed frame

vars == <<recvThrough, alive, localDisc, localFrame, cacheDisc, cacheLast,
          link, bound, recSrc>>

(***************************************************************************)
(* Min over a non-empty set of integers. CHOOSE ranges over integer frame    *)
(* values only (never over SURVIVORS), so it is symmetry-safe — though this   *)
(* module checks liveness and therefore declares no SYMMETRY anyway.          *)
(***************************************************************************)
MinI(S) == CHOOSE x \in S : \A y \in S : x <= y
Min2(a, b) == IF a <= b THEN a ELSE b
Max2(a, b) == IF a >= b THEN a ELSE b

(***************************************************************************)
(* GlobalMin: the agreed freeze frame F if the dropped slot drops — the        *)
(* minimum over ALL survivors of their received frame. Because frames are       *)
(* contiguous from 0, every survivor has a confirmed input at GlobalMin.        *)
(***************************************************************************)
GlobalMin == MinI({recvThrough[s] : s \in SURVIVORS})

(***************************************************************************)
(* The window floor for survivor s: frames strictly below it are LOCKED        *)
(* (discarded, no longer re-rollable). Mirrors adjust_gamestate's              *)
(* frame_to_load.max(current - max_prediction).                               *)
(***************************************************************************)
WindowFloor(s) ==
    IF bound[s] = NULL_FRAME THEN 0
    ELSE Max2(0, bound[s] - WINDOW)

Locked(s, g) == g < WindowFloor(s)

Committed(s, g) == recSrc[s][g] # NULL_FRAME

(***************************************************************************)
(* The set of OTHER survivors whose cached view survivor s folds. Production    *)
(* skips `!endpoint.is_running()` (dead) endpoints. The "Tombstone" candidate   *)
(* fix additionally retains a dead survivor's last gossiped term for a          *)
(* not-yet-mesh-agreed slot (the proposal under test).                          *)
(***************************************************************************)
SlotMeshAgreed(s) ==
    \* s considers D mesh-agreed-disconnected: s locally disconnected AND no
    \* folded running peer still reports it connected (the (true,false,_) arm).
    /\ localDisc[s]
    /\ \A o \in SURVIVORS \ {s} : alive[o] => cacheDisc[s][o]

FoldedSources(s) ==
    \* Tombstone (candidate fix #2): retain a DEAD survivor's last gossiped term
    \* for a not-yet-mesh-agreed slot, REGARDLESS of its disconnect bit — the
    \* relay residual leaves us with a dead origin's {connected, F} term (the
    \* origin died before gossiping its disconnect to us), so a tombstone that
    \* only kept DISCONNECTED dead terms would not even fix the relay. Keeping
    \* the connected-low term is exactly what regresses liveness (a dead laggy
    \* peer's {connected, F} pins a still-LIVE slot's confirmation forever).
    IF FIX_MODE = "Tombstone"
    THEN { o \in SURVIVORS \ {s} : alive[o] \/ ~SlotMeshAgreed(s) }
    ELSE { o \in SURVIVORS \ {s} : alive[o] }

(***************************************************************************)
(* remote_slot_confirmed_bound(s): the freeze-barrier bound on s's confirmed    *)
(* frame for the dropped slot. Faithful to the four production arms.            *)
(*   - localConn: D still connected in s's own local_connect_status.            *)
(*   - localLast: s's own receipt (connected) or freeze (disconnected).         *)
(*   - gossipMin: min folded cached last_frame.                                *)
(*   - anyConn: some folded peer still reports D connected.                     *)
(* Returns NULL_FRAME for the mesh-agreed (excluded) slot — D imposes no bound  *)
(* and confirmation may advance to MAX_FRAME (other slots, unmodeled, gate it). *)
(***************************************************************************)
ConfirmedBound(s) ==
    LET localConn == ~localDisc[s]
        localLast == localFrame[s]
        folded    == FoldedSources(s)
        anyConn   == \E o \in folded : ~cacheDisc[s][o]
    IN  IF folded = {}
        THEN IF localConn THEN localLast ELSE NULL_FRAME
        ELSE LET gossipMin == MinI({cacheLast[s][o] : o \in folded})
             IN  IF localConn THEN Min2(localLast, gossipMin)
                 ELSE IF anyConn THEN gossipMin
                 ELSE NULL_FRAME

(***************************************************************************)
(* The Baseline / Tombstone confirmation target: the freeze-barrier bound        *)
(* computed from the CACHES, promoted to MAX_FRAME when the slot is excluded      *)
(* (mesh-agreed — the frozen value carries it). Tombstone differs only in          *)
(* FoldedSources (it keeps dead survivors' caches), so it rides this same target.  *)
(***************************************************************************)
BaselineTarget(s) ==
    LET b == ConfirmedBound(s)
    IN IF b = NULL_FRAME THEN MAX_FRAME ELSE b

(***************************************************************************)
(* Candidate fix #3 ("MeshAgree", the sound one): confirmation of a               *)
(* not-yet-mesh-agreed slot may advance only to the MESH-ACKED FLOOR — the min,     *)
(* across the local view and every alive peer REACHABLE over a live link, of that   *)
(* peer's CURRENT floor (its receipt while it still sees D live, its freeze frame    *)
(* once it has dropped D). Two faithfulness points distinguish it from Baseline:      *)
(*                                                                                  *)
(*   1. It reads each peer's CURRENT floor (PeerFloor), an abstraction of a fresh     *)
(*      ack round, NOT the possibly-STALE per-endpoint cache the barrier folds. The    *)
(*      residual is precisely a survivor trusting a stale-high cache of a peer that     *)
(*      has since frozen D lower; a fresh ack cannot be stale.                          *)
(*   2. If ANY alive peer is currently UNREACHABLE (partitioned: the ack round           *)
(*      cannot complete), the survivor HOLDS — it does not advance past its current      *)
(*      bound — because an unheard peer might hold a lower freeze. A DEAD peer has         *)
(*      left the ack set (its endpoint is pruned), so it imposes no hold; the living       *)
(*      mesh floor is over the survivors that remain.                                       *)
(*                                                                                       *)
(* Once the slot is mesh-agreed-excluded, the frozen value carries it and the hold        *)
(* lifts (target MAX_FRAME). This is the design a production red-green cycle should        *)
(* implement (a per-slot ack/epoch on connect-status gossip); the wire format is the        *)
(* production choice this proof informs, not constrains.                                     *)
(***************************************************************************)
PeerFloor(o) == IF localDisc[o] THEN localFrame[o] ELSE recvThrough[o]

ReachableAlive(s) == { o \in SURVIVORS \ {s} : alive[o] /\ link[o][s] }

AllAliveReachable(s) == \A o \in SURVIVORS \ {s} : alive[o] => link[o][s]

MeshAckedFloor(s) ==
    MinI({PeerFloor(s)} \union {PeerFloor(o) : o \in ReachableAlive(s)})

CurBound(s) == IF bound[s] = NULL_FRAME THEN -1 ELSE bound[s]

MeshAgreeTarget(s) ==
    \* Partitioned: hold — an unheard alive peer might hold a lower freeze, and the
    \* ack round cannot complete. Safe default; the WF heal lifts it.
    IF ~AllAliveReachable(s) THEN CurBound(s)
    \* Mesh-agreed AND the local freeze has FULLY CONVERGED to the mesh-acked floor:
    \* the slot is excluded, every frame above the (now final) freeze is the same
    \* frozen value, so advancing to MAX_FRAME and locking is safe. Requiring the
    \* convergence (localFrame == MeshAckedFloor) is load-bearing: excluding while the
    \* freeze is still ABOVE the mesh floor would lock real/early-frozen frames that a
    \* later converge-down must rewrite — the discard-before-convergence half of the
    \* residual (a survivor that froze high off a stale cache after the low source died).
    ELSE IF SlotMeshAgreed(s) /\ localFrame[s] = MeshAckedFloor(s) THEN MAX_FRAME
    \* Otherwise cap by the fresh mesh-acked floor (never lock past a frame the freeze
    \* could still converge below).
    ELSE Min2(BaselineTarget(s), MeshAckedFloor(s))

BoundTarget(s) ==
    IF FIX_MODE = "MeshAgree" THEN MeshAgreeTarget(s) ELSE BaselineTarget(s)

(***************************************************************************)
(* The value (recorded SOURCE frame) survivor s writes for the dropped slot at  *)
(* frame g, given s's CURRENT freeze view: the frozen frame when s has          *)
(* concluded D disconnected and g is above the freeze, else the real frame g.   *)
(***************************************************************************)
RecordValue(s, g) ==
    IF localDisc[s] /\ g > localFrame[s] THEN localFrame[s] ELSE g

(***************************************************************************)
(* Type invariant.                                                          *)
(***************************************************************************)
TypeInvariant ==
    /\ recvThrough \in [SURVIVORS -> 0..MAX_FRAME]
    /\ alive \in [SURVIVORS -> BOOLEAN]
    /\ localDisc \in [SURVIVORS -> BOOLEAN]
    /\ localFrame \in [SURVIVORS -> Frame]
    /\ cacheDisc \in [SURVIVORS -> [SURVIVORS -> BOOLEAN]]
    /\ cacheLast \in [SURVIVORS -> [SURVIVORS -> Frame]]
    /\ link \in [SURVIVORS -> [SURVIVORS -> BOOLEAN]]
    /\ bound \in [SURVIVORS -> Frame]
    /\ recSrc \in [SURVIVORS -> [0..MAX_FRAME -> Frame]]

(***************************************************************************)
(* Initial state. The warmup phase (repro Phase 1: all links open, all survivors *)
(* confirm together) has completed: every survivor has gossiped its true receipt *)
(* to every other, so each cache holds the source's real receipt and nobody is   *)
(* disconnected. Nothing is confirmed yet (AdvanceConfirm catches up from here).  *)
(* recvThrough is the adversarial asymmetric-loss choice. Initialising the cache  *)
(* to the true receipt (not 0) is load-bearing: a survivor that drops a peer      *)
(* freezes at its queue-min over these caches, which must be >= GlobalMin — the   *)
(* asymmetry the residual needs comes from a (post-warmup) partition leaving a     *)
(* link's cache to go stale as the source's view changes, not from a cold cache.  *)
(* Links are chosen freely at Init (any post-warmup partition) and only heal.      *)
(***************************************************************************)
Init ==
    /\ recvThrough \in [SURVIVORS -> RECEIPTS]
    /\ alive = [s \in SURVIVORS |-> TRUE]
    /\ localDisc = [s \in SURVIVORS |-> FALSE]
    /\ localFrame = [s \in SURVIVORS |-> recvThrough[s]]
    /\ cacheDisc = [o \in SURVIVORS |-> [s \in SURVIVORS |-> FALSE]]
    /\ cacheLast = [o \in SURVIVORS |-> [s \in SURVIVORS |-> recvThrough[s]]]
    \* Links: any post-warmup partition (the FilterBus severs of the in-process repro),
    \* with the irrelevant self-links pinned up to avoid spurious initial states.
    \* Links only heal from here (Unblock), so the network monotonically stabilizes.
    /\ link \in [SURVIVORS -> [SURVIVORS -> BOOLEAN]]
    /\ \A s \in SURVIVORS : link[s][s]
    /\ bound = [s \in SURVIVORS |-> NULL_FRAME]
    /\ recSrc = [s \in SURVIVORS |-> [g \in 0..MAX_FRAME |-> NULL_FRAME]]

(***************************************************************************)
(* Action: src gossips its CURRENT local view of the dropped slot to obs        *)
(* (one connect-status vector entry). Requires src alive and the link up. obs   *)
(* merges into its cache via the faithful merge_peer_connect_status semantics:  *)
(*   - src reports disconnected, obs first learns: ADOPT src's frame.            *)
(*   - both disconnected: MIN (converge down).                                  *)
(*   - src reports connected, cache connected: MAX (monotone up).                *)
(*   - src reports connected, cache already disconnected: leave (no resurrect).  *)
(***************************************************************************)
Gossip(src, obs) ==
    /\ src # obs
    /\ alive[src]
    /\ alive[obs]
    /\ link[src][obs]
    /\ LET sd == localDisc[src]
           sf == localFrame[src]
           cd == cacheDisc[obs][src]
           cf == cacheLast[obs][src]
           newDisc == cd \/ sd
           newLast == IF sd /\ ~cd THEN sf                 \* first-learn disconnect: adopt
                      ELSE IF sd /\ cd THEN Min2(cf, sf)    \* both disconnected: min down
                      ELSE IF ~sd /\ ~cd THEN Max2(cf, sf)  \* both connected: max up
                      ELSE cf                               \* connected report, cache disc: no resurrect
       IN /\ cacheDisc' = [cacheDisc EXCEPT ![obs][src] = newDisc]
          /\ cacheLast' = [cacheLast EXCEPT ![obs][src] = newLast]
    /\ UNCHANGED <<recvThrough, alive, localDisc, localFrame, link, bound, recSrc>>

(***************************************************************************)
(* Action: src directly concludes the dropped slot disconnected (an explicit    *)
(* remove_player or a disconnect timeout). It freezes at its own current        *)
(* queue-min (its receipt, mined no lower than any folded lower cache). Models   *)
(* remove_player(D) on a survivor.                                              *)
(***************************************************************************)
QueueMin(s) ==
    LET folded == FoldedSources(s)
        base   == recvThrough[s]
    IN IF folded = {}
       THEN base
       ELSE Min2(base, MinI({cacheLast[s][o] : o \in folded}))

DetectDrop(s) ==
    /\ alive[s]
    /\ ~localDisc[s]
    /\ localDisc' = [localDisc EXCEPT ![s] = TRUE]
    /\ localFrame' = [localFrame EXCEPT ![s] = QueueMin(s)]
    /\ UNCHANGED <<recvThrough, alive, cacheDisc, cacheLast, link, bound, recSrc>>

(***************************************************************************)
(* Action: a survivor dies / is pruned (explicit remove_player by a peer, or a  *)
(* disconnect timeout). Removed from every fold immediately.                    *)
(***************************************************************************)
Die(s) ==
    /\ alive[s]
    /\ Cardinality({o \in SURVIVORS : alive[o]}) > 1   \* never empty the mesh
    /\ alive' = [alive EXCEPT ![s] = FALSE]
    /\ UNCHANGED <<recvThrough, localDisc, localFrame, cacheDisc, cacheLast,
                   link, bound, recSrc>>

(***************************************************************************)
(* Action: a partitioned (directed) link HEALS. Partitions are modeled as          *)
(* present POST-WARMUP (chosen by Init) and MONOTONICALLY HEALING — once a link is  *)
(* up it stays up. This is faithful (the in-process repro warms up with all links   *)
(* open, then severs and later re-opens) AND it makes the liveness obligation        *)
(* tractable: links monotonically reach all-up, so the network eventually           *)
(* stabilizes by construction — no adversarial Block/Unblock flap, so the plain      *)
(* `<>[] AllConfirmed` property suffices (no co-Buechi `<>[]Stable` antecedent).     *)
(* A partition that appears only LATER (after some confirmation) is captured by      *)
(* relabeling: the only confirmations that matter for the residual happen DURING     *)
(* the partition (the victim confirms past the freeze while it cannot hear the relay *)
(* lower it), and starting in that partition with those confirmations not-yet-done   *)
(* covers exactly that window. Weak fairness (in Fairness) forces every down link to *)
(* eventually heal — a real partition heals or the peer times out and dies.          *)
(***************************************************************************)
Unblock(src, dst) ==
    /\ src # dst
    /\ ~link[src][dst]
    /\ link' = [link EXCEPT ![src][dst] = TRUE]
    /\ UNCHANGED <<recvThrough, alive, localDisc, localFrame, cacheDisc,
                   cacheLast, bound, recSrc>>

(***************************************************************************)
(* Action: update_player_disconnects — fold the caches, and if the dropped      *)
(* slot now shows disconnected-and-lower than our local view, adopt the          *)
(* disconnect and mine our freeze frame DOWN, re-rolling every still-rollable    *)
(* (non-locked) committed frame above the new freeze to the frozen value.        *)
(* LOCKED (discarded) frames keep their stale recorded value — the residual.     *)
(***************************************************************************)
FoldedDisc(s) == \E o \in FoldedSources(s) : cacheDisc[s][o]

UpdateDisconnects(s) ==
    /\ alive[s]
    /\ FoldedDisc(s)                          \* some folded peer reports D disconnected
    /\ LET qmin == QueueMin(s)
           newDisc == TRUE
           newFrame == IF localDisc[s] THEN Min2(localFrame[s], qmin) ELSE qmin
       IN /\ \/ ~localDisc[s]                  \* first adopt
             \/ qmin < localFrame[s]           \* or a strictly-lower converge-down
          /\ localDisc' = [localDisc EXCEPT ![s] = newDisc]
          /\ localFrame' = [localFrame EXCEPT ![s] = newFrame]
          /\ recSrc' = [recSrc EXCEPT ![s] =
                 [g \in 0..MAX_FRAME |->
                     IF /\ recSrc[s][g] # NULL_FRAME      \* committed
                        /\ g > newFrame                   \* above the new freeze
                        /\ ~Locked(s, g)                  \* still re-rollable (not discarded)
                     THEN newFrame                        \* re-roll to frozen value
                     ELSE recSrc[s][g]]]
    /\ UNCHANGED <<recvThrough, alive, cacheDisc, cacheLast, link, bound>>

(***************************************************************************)
(* Action: AdvanceConfirm — confirm the dropped slot up to the freeze-barrier    *)
(* target, committing a recorded source-frame value for each newly confirmed     *)
(* frame. Under MeshAgree, a partitioned survivor holds (does not advance a       *)
(* not-yet-mesh-agreed slot). The commit is irreversible once the frame later     *)
(* falls below the window floor.                                                 *)
(***************************************************************************)
AdvanceConfirm(s) ==
    /\ alive[s]
    /\ LET target == BoundTarget(s)
           cur == CurBound(s)
       IN /\ target > cur                       \* genuine progress (MeshAgree holds = no progress)
          /\ bound' = [bound EXCEPT ![s] = target]
          /\ recSrc' = [recSrc EXCEPT ![s] =
                 [g \in 0..MAX_FRAME |->
                     IF g > cur /\ g <= target /\ recSrc[s][g] = NULL_FRAME
                     THEN RecordValue(s, g)
                     ELSE recSrc[s][g]]]
    /\ UNCHANGED <<recvThrough, alive, localDisc, localFrame, cacheDisc,
                   cacheLast, link>>

Next ==
    \/ \E src, obs \in SURVIVORS : Gossip(src, obs)
    \/ \E s \in SURVIVORS : DetectDrop(s)
    \/ \E s \in SURVIVORS : Die(s)
    \/ \E src, dst \in SURVIVORS : Unblock(src, dst)
    \/ \E s \in SURVIVORS : UpdateDisconnects(s)
    \/ \E s \in SURVIVORS : AdvanceConfirm(s)

(***************************************************************************)
(* Fairness for the liveness property. WEAK fairness suffices because partitions    *)
(* only HEAL (links are monotone-up), so there is no adversarial flap that could      *)
(* starve a continuously-enabled action — once the network has stabilized, every      *)
(* enabled progress action stays enabled until it fires, which is exactly the WF      *)
(* obligation. (Weak, not strong, fairness keeps TLC's liveness tableau small;        *)
(* strong fairness over this many actions blows the check up.) Per-survivor gossip,   *)
(* disconnect-folding, and confirmation advancement are each weakly fair; healing is  *)
(* one existential WF (each Unblock strictly reduces the down-link count, so a single  *)
(* obligation drives the network to fully connected). The Tombstone liveness FAILURE  *)
(* survives this: it is STRUCTURAL — a dead survivor's tombstone caps the              *)
(* confirmation TARGET below the living floor, so AdvanceConfirm makes no progress no  *)
(* matter how fairly it is scheduled.                                                 *)
(***************************************************************************)
Fairness ==
    /\ \A src, obs \in SURVIVORS : WF_vars(Gossip(src, obs))
    /\ \A s \in SURVIVORS : WF_vars(UpdateDisconnects(s))
    /\ \A s \in SURVIVORS : WF_vars(AdvanceConfirm(s))
    /\ WF_vars(\E src, dst \in SURVIVORS : Unblock(src, dst))

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* SAFETY: no two ALIVE survivors ever hold PERMANENTLY divergent recorded        *)
(* confirmed state for the dropped slot. Because recorded inputs are injective in   *)
(* the source frame, equal recorded source frames mean byte-identical confirmed     *)
(* state. The divergence is PERMANENT — a genuine desync, not the benign staggered  *)
(* -detection transient FreezeConvergence permits pre-convergence — exactly when     *)
(* the frame is LOCKED (irreversibly discarded below the window floor) on BOTH        *)
(* survivors: neither can ever re-roll it to re-converge. This is the headline        *)
(* desync-closing property: VIOLATED under Baseline (the residual reproduces) and     *)
(* holds under Tombstone / MeshAgree. (A divergence where one side is still rollable   *)
(* is either re-converged by ConvergeDown or, if it never settles, caught by the       *)
(* liveness property ConfirmationProgresses.)                                          *)
(***************************************************************************)
NoConfirmedDivergence ==
    \A s1, s2 \in SURVIVORS :
        (alive[s1] /\ alive[s2]) =>
            \A g \in 0..MAX_FRAME :
                (/\ Committed(s1, g) /\ Committed(s2, g)
                 /\ Locked(s1, g) /\ Locked(s2, g)) =>
                    recSrc[s1][g] = recSrc[s2][g]

(***************************************************************************)
(* SAFETY (single-survivor sharpening): once a survivor has mesh-agreed the slot   *)
(* disconnected (its freeze frame is final at the global min), every LOCKED frame   *)
(* it committed must hold the AGREED value — the real input at g for g at or below   *)
(* the freeze, the frozen value (= the freeze frame's input) above it. A locked       *)
(* REAL record above the freeze (recSrc = g > freeze) is precisely the residual: a    *)
(* frame confirmed+discarded with the dropped peer's real input before the freeze      *)
(* converged below it, now unrecoverable. Catches the victim WITHOUT needing a          *)
(* second survivor; VIOLATED under Baseline, holds under Tombstone / MeshAgree.         *)
(***************************************************************************)
LockedRecordMatchesFreeze ==
    \A s \in SURVIVORS :
        (alive[s] /\ SlotMeshAgreed(s)) =>
            \A g \in 0..MAX_FRAME :
                (Committed(s, g) /\ Locked(s, g)) =>
                    recSrc[s][g] = IF g <= localFrame[s] THEN g ELSE localFrame[s]

(***************************************************************************)
(* SAFETY sanity: a survivor never freezes the dropped slot below the true        *)
(* global min (it would repeat a value no peer confirmed), and never records a      *)
(* source frame above what it actually received.                                   *)
(***************************************************************************)
FreezeNeverBelowGlobalMin ==
    \A s \in SURVIVORS :
        localDisc[s] /\ localFrame[s] # NULL_FRAME => localFrame[s] >= GlobalMin

RecordedSourceInRange ==
    \A s \in SURVIVORS :
        \A g \in 0..MAX_FRAME :
            Committed(s, g) =>
                /\ recSrc[s][g] >= 0
                /\ recSrc[s][g] <= recvThrough[s]
                /\ recSrc[s][g] <= g

SafetyInvariant ==
    /\ TypeInvariant
    /\ NoConfirmedDivergence
    /\ LockedRecordMatchesFreeze
    /\ FreezeNeverBelowGlobalMin
    /\ RecordedSourceInRange

(***************************************************************************)
(* NON-VACUITY WITNESSES (used ONLY by DoubleFailureRelay_Witness.cfg — a demo      *)
(* config expected to REPORT VIOLATIONS). The two headline safety invariants above   *)
(* are universally-quantified implications; they would be VACUOUSLY true if their     *)
(* hypotheses (two alive survivors both LOCKING the same frame; a mesh-agreed         *)
(* survivor holding a LOCKED frame) were never reached. Each witness is the NEGATION  *)
(* of "the interesting state is reachable", so TLC reporting it VIOLATED proves the   *)
(* hypothesis IS reached under the MeshAgree (passing) config — i.e. the PASS is not  *)
(* hollow. The mesh-agreed witness is deliberately NON-DEGENERATE: it requires >= 2   *)
(* alive survivors and a genuine cross-survivor `cacheDisc` agreement, ruling out the *)
(* trivial all-peers-dead path that makes `SlotMeshAgreed`'s `\A` vacuous.            *)
(***************************************************************************)
WitnessTwoSurvivorsLockSameFrame ==
    ~(\E s1, s2 \in SURVIVORS, g \in 0..MAX_FRAME :
         /\ s1 # s2 /\ alive[s1] /\ alive[s2]
         /\ Committed(s1, g) /\ Committed(s2, g)
         /\ Locked(s1, g) /\ Locked(s2, g))

WitnessMeshAgreedWithLockedNonDegenerate ==
    ~(\E s \in SURVIVORS :
         /\ alive[s] /\ SlotMeshAgreed(s)
         /\ Cardinality({o \in SURVIVORS : alive[o]}) >= 2
         /\ \E o \in SURVIVORS : o # s /\ alive[o] /\ cacheDisc[s][o]
         /\ \E g \in 0..MAX_FRAME : Committed(s, g) /\ Locked(s, g))

(***************************************************************************)
(* LIVENESS: every alive survivor eventually reaches a stable fully-confirmed      *)
(* fixpoint for the dropped slot — its confirmation advances to its proper          *)
(* target (its own receipt while the slot is live, or MAX_FRAME once the slot is     *)
(* mesh-agreed and excluded). VIOLATED under Tombstone (a dead survivor's stale       *)
(* -low tombstone pins live confirmation forever) and holds under Baseline /          *)
(* MeshAgree.                                                                         *)
(*                                                                                   *)
(* "Proper target" is the LIVING MESH FLOOR — the min receipt over survivors that         *)
(* are still ALIVE (a peer's confirmed frame for a slot is GGPO-bounded by the SLOWEST     *)
(* surviving peer's receipt, never its own; a dead peer leaves the floor). Once the slot   *)
(* mesh-agrees-disconnects it is excluded and confirmation runs to MAX_FRAME (>= the floor *)
(* trivially). So the obligation is: eventually, every alive survivor has confirmed at      *)
(* least to the living mesh floor. Baseline and MeshAgree both reach it; Tombstone pins a    *)
(* survivor BELOW it forever (a dead laggard's retained low term), which is the violation.   *)
(***************************************************************************)
LivingFloor == MinI({recvThrough[o] : o \in {x \in SURVIVORS : alive[x]}})

FullyConfirmed(s) ==
    \/ ~alive[s]
    \/ (bound[s] # NULL_FRAME /\ bound[s] >= LivingFloor)

(***************************************************************************)
(* Because partitions only HEAL (links are monotone-up; see Unblock) and Unblock   *)
(* is weakly fair, the network stabilizes by construction, so the obligation is the  *)
(* plain "eventually, forever, every alive survivor is fully confirmed." It          *)
(* distinguishes the fixes: under Baseline and MeshAgree the mesh converges and       *)
(* every survivor reaches the living floor; under Tombstone a dead laggard's retained *)
(* low term caps a survivor below the living floor FOREVER — independent of the        *)
(* (fully healed) network — so the property is violated.                              *)
(***************************************************************************)
ConfirmationProgresses == <>[](\A s \in SURVIVORS : FullyConfirmed(s))

THEOREM Spec => []SafetyInvariant

=============================================================================
