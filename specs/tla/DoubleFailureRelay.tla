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
(* a CONSTANT `FIX_MODE`. The four ORIGINAL values below were the S41/S42/S44   *)
(* arbitration; the S47 async LADDER (AsyncAckStale / AsyncAckGossip /          *)
(* AsyncAckTwoPhase) and the S48 CERTIFIED-SOUND culmination (AsyncAckSound)    *)
(* are documented at the AsyncMode definition. Each value has its own .cfg:     *)
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
(*   - "InheritedFloor" — candidate fix #4 (the CACHE-ONLY / NO-WIRE shortcut): *)
(*                   the cheapest, most TEMPTING design — snapshot a departed     *)
(*                   connected source's last cached low term (bounding the        *)
(*                   confirm target AND the freeze), release it only on FRESH     *)
(*                   gossip from every remaining alive peer reporting the slot     *)
(*                   connected above it. No wire change, no peer-receipt oracle.   *)
(*                   MACHINE-DISPROVEN UNSOUND: NoConfirmedDivergence is VIOLATED  *)
(*                   via the CORROBORATE-THEN-DROP race (a peer corroborates       *)
(*                   "healthy", the observer releases, then that peer drops the    *)
(*                   slot low while its disconnect gossip is still in flight, so   *)
(*                   the observer confirms+locks real inputs above the freeze).    *)
(*                   Passive gossip corroboration is point-in-time and reversible  *)
(*                   by the corroborator's own later drop, so NO cache-only        *)
(*                   release is sound — the formal proof that the fix needs a      *)
(*                   FRESH-ACK ROUND + DROP-EPOCH, not passive gossip. (Run via    *)
(*                   DoubleFailureRelay_InheritedFloor.cfg — expected to FAIL on   *)
(*                   safety; the third documented dead-end, the no-wire shortcut.) *)
(*                                                                          *)
(*   - The S47 async LADDER + S48 SOUND mode (see the AsyncMode definition for the *)
(*     full prose): "AsyncAckStale" / "AsyncAckGossip" / "AsyncAckTwoPhase"          *)
(*     discharge MeshAgree's two idealizations (modeling a concrete in-flight ack     *)
(*     round + per-observer death) and are ALL machine-DISPROVEN UNSOUND (each its    *)
(*     own expected-FAIL demo .cfg); "AsyncAckSound" is the CERTIFIED-SOUND           *)
(*     implementable culmination — the AsyncAckStale machinery with the SINGLE delta  *)
(*     of a PESSIMISTIC queue-min report — which PASSES safety AND liveness           *)
(*     (DoubleFailureRelay_AsyncAckSound.cfg). It is the implementable analog of the  *)
(*     idealized MeshAgree positive: the design a production red-green cycle should    *)
(*     implement (S47 blueprint: pessimistic report + fresh-ack round + no            *)
(*     partition-hold, on the S46 connect-status gossip).                            *)
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
(* SCOPE OF THE RESULTS (honest bounds — what each claim does and does NOT       *)
(* prove; established in an adversarial faithfulness review):                    *)
(*                                                                          *)
(*   - The NEGATIVE results are UNCONDITIONAL within the modeled world. Baseline  *)
(*     -unsafe, Tombstone-illiveness, and InheritedFloor-unsafe are demonstrated  *)
(*     by REACHABLE counterexamples, and every abstraction here is conservative   *)
(*     (it makes the model safer/more-restrictive than production), so a reachable *)
(*     violation is a real one. These three are the load-bearing arbitration       *)
(*     outcomes: they rule out, respectively, doing nothing, the dead-survivor     *)
(*     tombstone (liveness), and the cache-only no-wire shortcut (safety, via the  *)
(*     corroborate-then-drop race) — establishing that the sound fix needs a       *)
(*     fresh-ack ROUND + drop-epoch, not passive gossip.                          *)
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
(*                                                                          *)
(*   - The S48 POSITIVE result (AsyncAckSound sound) models a CONCRETE in-flight     *)
(*     ack round (the observer reads the snapshot `ackFloor`, not a live `PeerFloor`) *)
(*     and PER-OBSERVER death (`pruned`, the skewed timeout) — GENUINELY discharging  *)
(*     idealization (b) — and still PASSES safety AND liveness. So unlike MeshAgree   *)
(*     (an idealized POLICY), AsyncAckSound is a directly implementable design. The   *)
(*     decisive fold change is the PESSIMISTIC report (a relay surfaces a            *)
(*     folded/departed low immediately): flipping ONLY the report from own-floor to  *)
(*     pessimistic flips the disproven AsyncAckStale FAIL->PASS (verified at a        *)
(*     matched EPOCH_MAX=0), so it is what closes idealization (a)'s DEPARTED-LOW     *)
(*     facet, with NO epoch freshness gate. HONEST PRECISION (do not over-read as     *)
(*     "the report alone discharges (a)"): safety has TWO faithful load-bearing       *)
(*     ingredients — (1) the pessimistic report applied FROM WARMUP (the `GlobalMin`  *)
(*     `ackFloor` seed: every observer holds a pessimistic ack <= GlobalMin before    *)
(*     any drop; reverting it to the own-receipt seed reproduces the residual,        *)
(*     mutation-pinned) and (2) `FreezeNeverBelowGlobalMin`, which makes a stale       *)
(*     warmup ack a permanently-valid lower bound. So (a)'s STALE-IN-FLIGHT facet is   *)
(*     closed by the GlobalMin floor + warmup seed, NOT by ack freshness. The shared   *)
(*     model bound is therefore the warm-GlobalMin convention (every survivor has      *)
(*     heard every receipt AND holds a pessimistic ack before any drop); a cold-cache  *)
(*     "never received a pessimistic ack" world is out of scope and is where the       *)
(*     drop-epoch / fresh-round COMMITMENT additionally binds — plus the non-hot-join  *)
(*     fold arms and the monotone-healing-partition liveness premise.                 *)
(***************************************************************************)

EXTENDS Integers, FiniteSets, TLC

CONSTANTS
    SURVIVORS,     \* observing survivors, e.g. {a, b, c} (a=us, b=dying origin, c=relay)
    MAX_FRAME,     \* maximum frame number for model checking
    NULL_FRAME,    \* sentinel "no frame" (-1 in impl)
    WINDOW,        \* prediction window: frames > bound - WINDOW are still re-rollable
    RECEIPTS,      \* allowed per-survivor receipts of the dropped slot (adversarial at Init)
    FIX_MODE,      \* "Baseline" | "Tombstone" | "MeshAgree" | "InheritedFloor"
                   \*   | "AsyncAckTwoPhase" | "AsyncAckGossip" | "AsyncAckStale" (S47 — in-flight ack
                   \*   + per-observer death) | "AsyncAckSound" (S48 — the CERTIFIED-SOUND implementable
                   \*   mode, sound in the WARM-cache scope) | "AsyncAckSoundFresh" (S49 — the
                   \*   COLD-cache-sound mode: pessimistic report + a fresh-ack-ROUND hold that does
                   \*   NOT trust the cold seed; the SINGLE delta from AsyncAckSound that flips cold
                   \*   FAIL -> PASS)
    EPOCH_MAX,     \* (AsyncAckTwoPhase/Gossip/Stale) max per-slot drop-epoch generation (small, e.g. 2)
    COLD_CACHE     \* (S49) BOOLEAN: model the COLD-CACHE corner S48 named out-of-scope — an observer
                   \*   that NEVER received a pessimistic ack (<= GlobalMin) before the drop. It seeds
                   \*   `ackFloor` at the own-receipt-HIGH value instead of the warm `GlobalMin`,
                   \*   removing the S48 warmup-seed crutch so the in-flight STALE-HIGH facet of
                   \*   idealization (a) is reopened (the warm seed no longer closes it). FALSE in every
                   \*   pre-S49 cfg (the established warm-GlobalMin convention) — orthogonal to FIX_MODE,
                   \*   so the SAME mode can be run warm vs cold to isolate the seed as the variable.

ASSUME SURVIVORS # {}
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME NULL_FRAME \notin 0..MAX_FRAME
ASSUME WINDOW \in Nat /\ WINDOW >= 1
ASSUME RECEIPTS \subseteq (0..MAX_FRAME) /\ RECEIPTS # {}
ASSUME FIX_MODE \in {"Baseline", "Tombstone", "MeshAgree", "InheritedFloor",
                     "AsyncAckStale", "AsyncAckGossip", "AsyncAckTwoPhase", "AsyncAckSound",
                     "AsyncAckSoundFresh"}
ASSUME EPOCH_MAX \in Nat
ASSUME COLD_CACHE \in BOOLEAN

\* The S47/S48 modes that discharge the MeshAgree idealizations (a) instantaneous-
\* fresh-ack and (b) synchronized-death by modeling a CONCRETE in-flight ack round +
\* per-observer (async) death. The first three form the S47 soundness LADDER of increasingly
\* sophisticated IMPLEMENTABLE epoch mechanisms, ALL machine-DISPROVEN UNSOUND (each has its
\* own demo .cfg expected to FAIL on a safety invariant); the fourth, AsyncAckSound, is the
\* S48 CERTIFIED-SOUND culmination that PASSES safety AND liveness (its own registered cfg):
\*   - AsyncAckStale  : in-flight ack, NO epoch freshness gate, OWN-floor report -> UNSOUND.
\*   - AsyncAckGossip : in-flight ack + PASSIVE gossip-epoch freshness gate
\*                      (the S46 wire epoch, compared against the gossip cache), OWN-floor
\*                      report                                            -> UNSOUND
\*                      (the epoch-bump gossip RACES the observer's lock).
\*   - AsyncAckTwoPhase : in-flight ack + the gossip-epoch gate + a TWO-PHASE
\*                      announce/HOLD-commit COMMITMENT (the lowering peer announces its
\*                      epoch bump and HOLDS its freeze until every folding observer has
\*                      heard it), OWN/announced-floor report             -> UNSOUND
\*                      (the DEPARTED-LOW race: per-observer death lets the relay adopt the
\*                      dead origin's low while the victim, having pruned the origin, has
\*                      forgotten it — the S44 InheritedFloor obstruction, now structural).
\*   - AsyncAckSound  : in-flight ack + reachability-gated fresh-ack round (NO partition-hold)
\*                      + the PESSIMISTIC QUEUE-MIN report (the relay reports the min over its
\*                      own floor AND every folded source's cached last_frame, surfacing a
\*                      departed origin's low IMMEDIATELY)                -> SOUND
\*                      (safety + liveness PASS). The pessimistic report is the decisive delta
\*                      from the disproven AsyncAckStale (flipping ONLY the report flips
\*                      FAIL->PASS, verified at a matched EPOCH_MAX=0). Because a pessimistic
\*                      floor is monotone-NON-DECREASING (it RISES only when the source prunes a
\*                      departed origin, and then the source ALSO freezes/confirms high), every
\*                      pessimistic ack snapshot is <= the source's eventual freeze, so it is a
\*                      sound lower bound with NO epoch freshness gate or two-phase commitment.
\*                      TWO load-bearing ingredients, both faithful (do NOT read this as
\*                      "the report alone"): (1) the pessimistic report, applied FROM WARMUP —
\*                      the `GlobalMin` `ackFloor` Init seed encodes a completed warmup round in
\*                      which every observer obtained a pessimistic ack (= GlobalMin) BEFORE any
\*                      drop/partition; reverting that seed to the own-receipt value (a
\*                      NON-pessimistic warmup) reproduces the residual (mutation-pinned), so the
\*                      warmup propagation is genuinely load-bearing, not free; and (2) the
\*                      `FreezeNeverBelowGlobalMin` floor, which makes even a stale warmup ack a
\*                      permanently-valid lower bound. So idealization (b) is GENUINELY discharged
\*                      (per-observer `pruned`); idealization (a)'s DEPARTED-LOW facet is closed by
\*                      the report, but its STALE-IN-FLIGHT facet is closed by the GlobalMin floor +
\*                      the warmup-pessimistic seed, NOT by ack freshness. This is S47's blueprint
\*                      (pessimistic report + fresh-ack round + no partition-hold) made concrete —
\*                      with the honest sharpening that the fresh-ack round must have DELIVERED a
\*                      pessimistic ack to every observer (the warm-GlobalMin premise); the
\*                      cold-cache corner, where it has not, is DISCHARGED in S49 by AsyncAckSoundFresh
\*                      (COLD_CACHE=TRUE): the cold fix is the OBSERVER-side fresh-ack ROUND — NULL-seed
\*                      the ack and HOLD until a pessimistic ack is RECEIVED (do not trust the cold
\*                      cache) — NOT a sender-side epoch/commitment. The epoch binds only on the further
\*                      cold-GOSSIP-cache / mid-game-drop sliver (a freshness gate on the received ack).
\* The discharge of idealization (b) is itself the key insight: under SYNCHRONIZED death
\* (MeshAgree's idealization) the low-receipt origin is pruned from EVERY fold at once, so
\* no relay adopts its low while a victim has lost it — which is why the idealized MeshAgree
\* PASSES and the three OWN-floor async models do not. The pessimistic report is what lets a
\* per-observer-death async model PASS again: whichever survivor still folds the departed
\* origin reports its low, so no observer can confirm past a frame a survivor will freeze
\* below. The S46 passive drop-epoch is thus NECESSARY-on-the-wire-but-not-SUFFICIENT-as-a-
\* -gate on the player side (AsyncAckGossip); the sufficient mechanism is the pessimistic
\* report (AsyncAckSound). The MeshAgree POLICY positive (its cfg) still stands.
\* AsyncMode gates ALL the per-observer-death + in-flight-ack machinery so the four
\* NON-async modes (Baseline/Tombstone/MeshAgree/InheritedFloor) stay state-identical (their
\* async variables stay pinned at Init forever).
AsyncMode == FIX_MODE \in {"AsyncAckStale", "AsyncAckGossip", "AsyncAckTwoPhase", "AsyncAckSound",
                           "AsyncAckSoundFresh"}

\* The certified-sound implementable mode (S48). Gates the AsyncSoundTarget confirmation rule
\* (pessimistic acked floor, NO passive epoch gate, NO two-phase commitment). It reuses the
\* AsyncAckStale machinery (in-flight ack, per-observer death, reachability-gated round) and
\* changes ONLY the reported floor — isolating the pessimistic report as the load-bearing fix
\* IN THE WARM-CACHE SCOPE (the Init `GlobalMin` ackFloor seed). With COLD_CACHE it is the
\* COLD NEGATIVE: pessimistic report alone, stripped of the warm seed, reopens the
\* stale-in-flight residual (run DoubleFailureRelay_AsyncAckSound_Cold.cfg — expected FAIL).
SoundMode == FIX_MODE = "AsyncAckSound"

\* S49 — the COLD-CACHE-SOUND mode. The cold corner's residual is that an observer TRUSTS its
\* COLD ackFloor SEED (the relay's own high receipt) and advances on it before receiving a
\* genuine pessimistic ack. AsyncAckSoundFresh closes it by NOT trusting the seed: its ackFloor
\* is NULL-seeded ("no ack received"), and the confirmation HOLDS for any folded reachable peer
\* whose ack has not yet arrived (AsyncSoundFreshTarget). Once the relay's fresh ack lands it
\* carries the relay's PESSIMISTIC floor (PessimisticReport — surfacing the departed origin's
\* low it still folds), so the observer holds at the global min. This is S47's "fresh-ack ROUND
\* postdating the observer's intent" made concrete: the observer must COMPLETE the round (get a
\* received ack), not read a stale/cold cache. PASSES safety+liveness COLD
\* (DoubleFailureRelay_AsyncAckSoundFresh_Cold.cfg) — the implementable cold-robust analog of
\* the idealized MeshAgree policy. Honest scope: this model colds the ACK seed while keeping the
\* gossip CACHE warm (the relay always folds the origin's low — the faithful drop-at-Init
\* convention), so one received pessimistic ack suffices; a cold-GOSSIP-cache / mid-game-drop
\* world (where the relay's own cache is also cold) is a further corner needing the epoch
\* freshness gate, out of this model's scope.
SoundFreshMode == FIX_MODE = "AsyncAckSoundFresh"

\* Pessimistic queue-min report (AckReportFloor): the modes whose fresh ack carries the min over
\* the source's own floor AND every folded source's cache (surfacing a departed origin's low
\* immediately), rather than the source's own floor. The decisive fold delta.
PessimisticReport == FIX_MODE \in {"AsyncAckSound", "AsyncAckSoundFresh"}

\* The two-phase announce/HOLD-commit mechanism (the `announced` variable + the AnnounceLower
\* action + the LowerSafe gate on the lowering actions). AsyncAckTwoPhase (own-floor report)
\* is UNSOUND via the departed-low race (and fails cold too — the report matters).
TwoPhase == FIX_MODE = "AsyncAckTwoPhase"

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
    recSrc,        \* [SURVIVORS -> [0..MAX_FRAME -> Frame]]: recorded source frame per committed frame
    floor,         \* [SURVIVORS -> Frame] (InheritedFloor mode): inherited freeze-floor commitment
                   \*   (NULL = none). While non-NULL, s must not confirm D past floor[s]. Snapshotted
                   \*   SYNCHRONOUSLY when a CONNECTED folded source leaves s's fold (Die), from that
                   \*   source's last cached term — the production "do not forget a departed low term."
    corrob,        \* [SURVIVORS -> [SURVIVORS -> BOOLEAN]] (InheritedFloor mode): corrob[s][o] = since
                   \*   floor[s] was armed, o sent s FRESH gossip (a Gossip event, NOT a stale cache read)
                   \*   reporting D CONNECTED above floor[s]. Releasing floor[s] requires every alive
                   \*   folded source to corroborate — the event-driven "fresh ack" that closes the
                   \*   S42 instantaneous-ack idealization without reading any peer's true receipt.
    \* ----- S45 (AsyncAckTwoPhase / AsyncAckStale) machinery; pinned at Init in the other modes -----
    gone,          \* [SURVIVORS -> BOOLEAN]: s has ACTUALLY stopped (true death). Discharges
                   \*   idealization (b): unlike the global `alive` flag, death is observed
                   \*   per-observer via `pruned` (below) at skewed times. `gone[s]` is the ground
                   \*   truth; gossip/ack SEND from s requires `~gone[s]`.
    pruned,        \* [SURVIVORS -> [SURVIVORS -> BOOLEAN]]: pruned[obs][src] = obs has timed-out and
                   \*   pruned src from its own fold. May flip TRUE only once gone[src] (the disconnect
                   \*   timeout), independently per observer — different observers prune at skewed times
                   \*   (the per-observer death idealization (b)). In the async modes the folds iterate
                   \*   `~pruned[s][o]` instead of the global `alive[o]`.
    slotEpoch,     \* [SURVIVORS -> 0..EPOCH_MAX]: s's per-slot drop generation. BUMPED whenever s
                   \*   strictly LOWERS its floor (DetectDrop, or a lowering UpdateDisconnects). The
                   \*   epoch is what turns an ack into a COMMITMENT: an ack carrying epoch E asserts
                   \*   "I will not be below my floor for epoch E"; a later floor-lower bumps to E+1 and
                   \*   the old ack is superseded.
    ackFloor,      \* [SURVIVORS -> [SURVIVORS -> Frame]]: ackFloor[obs][src] = obs's last-RECEIVED ack
                   \*   floor from src (src's PeerFloor at the moment SendAck fired). In-flight latency:
                   \*   src may then LOWER its floor without a new SendAck, leaving this stale-HIGH.
    ackEpoch,      \* [SURVIVORS -> [SURVIVORS -> 0..EPOCH_MAX]]: epoch carried by that last ack
                   \*   (src's slotEpoch at SendAck time). Freshness gate: the ack is fresh iff this
                   \*   equals obs's gossip-tracked latest epoch cacheEpoch[obs][src].
    cacheEpoch,    \* [SURVIVORS -> [SURVIVORS -> 0..EPOCH_MAX]]: obs's gossip-tracked latest epoch from
                   \*   src (merged monotone-up in Gossip). AsyncAckGossip/Commit count an ack as fresh
                   \*   only when ackEpoch >= cacheEpoch; AsyncAckStale ignores this gate.
    announced      \* [SURVIVORS -> Frame] (AsyncAckTwoPhase two-phase only): the pending-low frame s has
                   \*   ANNOUNCED it will lower to (NULL = nothing pending). On AnnounceLower s bumps +
                   \*   gossips its drop-epoch and PUBLISHES this pending-low as its reported floor
                   \*   (PeerFloor reads it), so any FRESH re-ack already carries the low — and the
                   \*   COMMIT (DetectDrop / lowering UpdateDisconnects) is gated on LowerSafe (every
                   \*   folding observer has heard the epoch bump or is partitioned/holding), so an
                   \*   observer with a not-yet-refreshed stale-HIGH ack has already gone stale-and-held
                   \*   before s actually lowers. Pinned NULL in every non-AsyncAckTwoPhase mode.

vars == <<recvThrough, alive, localDisc, localFrame, cacheDisc, cacheLast,
          link, bound, recSrc, floor, corrob,
          gone, pruned, slotEpoch, ackFloor, ackEpoch, cacheEpoch, announced>>

\* The S47 async variables, as a tuple — left UNCHANGED by every original-mode action
\* (so the four original modes keep them pinned at their Init values, contributing a
\* factor of 1 to the state space). The async actions touch them explicitly.
asyncVars == <<gone, pruned, slotEpoch, ackFloor, ackEpoch, cacheEpoch, announced>>

(***************************************************************************)
(* Min over a non-empty set of integers. CHOOSE ranges over integer frame    *)
(* values only (never over SURVIVORS), so it is symmetry-safe — though this   *)
(* module checks liveness and therefore declares no SYMMETRY anyway.          *)
(***************************************************************************)
MinI(S) == CHOOSE x \in S : \A y \in S : x <= y
Min2(a, b) == IF a <= b THEN a ELSE b
Max2(a, b) == IF a >= b THEN a ELSE b

(***************************************************************************)
(* Survivor-permutation symmetry. SOUND only for SAFETY-only checks (TLC      *)
(* symmetry reduction is unsound for liveness, so the liveness-checking cfgs   *)
(* — DoubleFailureRelay.cfg / the *_AsyncAckSound*.cfg PASS cfgs — must NOT     *)
(* declare SYMMETRY). MinI's CHOOSE ranges over integer frames only (never     *)
(* over SURVIVORS), so it is symmetry-safe. Used by the large-state-space      *)
(* SAFETY-only demo cfgs (e.g. the S49 cold-mode safety cross-checks) to make   *)
(* their state space tractable.                                                *)
(***************************************************************************)
RelaySymmetry == Permutations(SURVIVORS)

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
(***************************************************************************)
(* "o is in s's fold" — production's `endpoint.is_running()` membership. In the *)
(* four original modes this is the GLOBAL alive flag (idealization (b),          *)
(* synchronized death). In the S45 async modes it is s's OWN per-observer prune   *)
(* view `~pruned[s][o]` (death is observed at skewed times). Gating on AsyncMode  *)
(* keeps the original four modes byte-identical (they never read `pruned`).       *)
(***************************************************************************)
InFold(s, o) == IF AsyncMode THEN ~pruned[s][o] ELSE alive[o]

SlotMeshAgreed(s) ==
    \* s considers D mesh-agreed-disconnected: s locally disconnected AND no
    \* folded running peer still reports it connected (the (true,false,_) arm).
    /\ localDisc[s]
    /\ \A o \in SURVIVORS \ {s} : InFold(s, o) => cacheDisc[s][o]

FoldedSources(s) ==
    \* Tombstone (candidate fix #2): retain a DEAD survivor's last gossiped term
    \* for a not-yet-mesh-agreed slot, REGARDLESS of its disconnect bit — the
    \* relay residual leaves us with a dead origin's {connected, F} term (the
    \* origin died before gossiping its disconnect to us), so a tombstone that
    \* only kept DISCONNECTED dead terms would not even fix the relay. Keeping
    \* the connected-low term is exactly what regresses liveness (a dead laggy
    \* peer's {connected, F} pins a still-LIVE slot's confirmation forever).
    CASE FIX_MODE = "Tombstone" ->
            { o \in SURVIVORS \ {s} : alive[o] \/ ~SlotMeshAgreed(s) }
      \* Async modes: per-observer fold membership (s's own prune view), discharging (b).
      [] AsyncMode ->
            { o \in SURVIVORS \ {s} : ~pruned[s][o] }
      [] OTHER ->
            { o \in SURVIVORS \ {s} : alive[o] }

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
PeerFloor(o) ==
    \* AsyncAckTwoPhase two-phase: while o has ANNOUNCED a pending lower (but not yet
    \* committed it), o already PUBLISHES that pending-low as its floor — so any fresh
    \* (current-epoch) ack o gives already carries the low value, never the stale high.
    IF TwoPhase /\ announced[o] # NULL_FRAME THEN announced[o]
    ELSE IF localDisc[o] THEN localFrame[o] ELSE recvThrough[o]

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

(***************************************************************************)
(* S47 — the IMPLEMENTABLE-mechanism LADDER (AsyncAckStale / AsyncAckGossip /          *)
(* AsyncAckTwoPhase), ALL machine-disproven UNSOUND. This shared confirmation rule     *)
(* discharges the two idealizations the MeshAgree proof assumed away:               *)
(*                                                                                 *)
(*   (a) INSTANTANEOUS FRESH ACK -> a CONCRETE in-flight ack round. Each observer  *)
(*       o holds, per source p, the LAST-RECEIVED ack (ackFloor[o][p],            *)
(*       ackEpoch[o][p]) — set by a SendAck(p,o) action reading p's CURRENT floor  *)
(*       AT SEND TIME. p may then LOWER its floor WITHOUT a new ack, leaving o's    *)
(*       ack stale-HIGH with an old epoch. This is the staleness MeshAgree's        *)
(*       PeerFloor(o) read elided.                                                  *)
(*   (b) SYNCHRONIZED DEATH -> per-observer prune (`pruned[o][p]`, fold membership *)
(*       above), so observers disagree on who is alive during the timeout skew.     *)
(*                                                                                  *)
(* The DROP-EPOCH (slotEpoch) is what turns a snapshot into a COMMITMENT: an ack     *)
(* carries the source's epoch; the source bumps its epoch when it lowers its floor;  *)
(* the observer tracks the latest epoch it has heard via gossip (cacheEpoch).        *)
(*                                                                                   *)
(* AsyncAckTwoPhase advances a not-yet-mesh-agreed slot only to the min, over its       *)
(* folded non-pruned peers p, of ackFloor[o][p], counting p ONLY IF the ack is        *)
(* FRESH (ackEpoch[o][p] == cacheEpoch[o][p]) AND not stale-superseded                *)
(* (ackEpoch[o][p] >= cacheEpoch[o][p]). If any folded non-pruned peer lacks a fresh   *)
(* ack, or o is partitioned from any folded non-pruned peer, o HOLDS. AsyncAckStale     *)
(* is identical EXCEPT it drops the freshness gate (acts on the last ack regardless     *)
(* of epoch) — the control that proves the epoch commitment is load-bearing.            *)
(***************************************************************************)
AsyncReachableFolded(s) == { o \in SURVIVORS \ {s} : ~pruned[s][o] /\ link[o][s] }

\* Every folded (non-pruned) peer is reachable to s — the ack round can complete.
AsyncAllFoldedReachable(s) == \A o \in SURVIVORS \ {s} : ~pruned[s][o] => link[o][s]

\* Is s's last ack from p FRESH (its epoch matches the latest epoch s has heard from p
\* via gossip)? A stale ack carries an OLDER epoch than cacheEpoch (p has since lowered
\* and re-gossiped a higher epoch that s merged). AsyncAckStale ignores this gate.
AckFresh(s, p) ==
    IF FIX_MODE = "AsyncAckStale" THEN TRUE
    ELSE ackEpoch[s][p] >= cacheEpoch[s][p]

\* The floor s may advance to from its non-pruned folded peers' acks: the min over the
\* local own term (PeerFloor) and every folded non-pruned peer's last ackFloor. s's OWN
\* pessimism (folding its own cached sources) is already in BaselineTarget(s) — AsyncAckTarget
\* takes min(BaselineTarget(s), AsyncAckedFloor(s)) — so the decisive pessimism for the cold
\* race is in the PEERS' acks (AckReportFloor deposited by SendAck), which surface a departed
\* origin's low to s; the local term here stays PeerFloor (byte-identical across all modes).
AsyncAckedFloor(s) ==
    MinI({PeerFloor(s)} \union {ackFloor[s][o] : o \in AsyncReachableFolded(s)})

\* Does EVERY folded non-pruned peer have a FRESH ack? (Commit holds otherwise.)
AsyncAllAcksFresh(s) ==
    \A o \in SURVIVORS \ {s} : ~pruned[s][o] => AckFresh(s, o)

AsyncAckTarget(s) ==
    \* Partitioned from a folded non-pruned peer: hold (ack round cannot complete).
    IF ~AsyncAllFoldedReachable(s) THEN CurBound(s)
    \* Any folded non-pruned peer lacks a fresh, current-epoch ack: hold. (For
    \* AsyncAckStale this is vacuously true, so the stale mode never holds on freshness —
    \* it acts on the last ack and reintroduces the in-flight race.)
    ELSE IF ~AsyncAllAcksFresh(s) THEN CurBound(s)
    \* Mesh-agreed AND fully converged to the acked floor: excluded, advance to MAX.
    ELSE IF SlotMeshAgreed(s) /\ localFrame[s] = AsyncAckedFloor(s) THEN MAX_FRAME
    \* Otherwise cap by the (committed) acked floor — never lock past a frame a peer's
    \* commitment says it may still converge below.
    ELSE Min2(BaselineTarget(s), AsyncAckedFloor(s))

(***************************************************************************)
(* S48 — the CERTIFIED-SOUND implementable mode ("AsyncAckSound"). The SINGLE      *)
(* delta from the disproven AsyncAckStale is the PESSIMISTIC queue-min report: a    *)
(* survivor's fresh ack carries the lowest frame it could STILL freeze D to given   *)
(* everything it currently folds — the min over its OWN floor (receipt while live,  *)
(* freeze once dropped) AND every non-pruned folded source's cached last_frame —     *)
(* rather than its own floor alone. This surfaces a DEPARTED origin's low the         *)
(* instant it is in the relay's fold, BEFORE the relay commits its own freeze, which   *)
(* is exactly what the AsyncAckStale/Gossip own-floor report omitted (the gossip race) *)
(* and what the AsyncAckTwoPhase announce/freeze report omitted (the departed-low      *)
(* race). It is S47's "relay reports its pessimistic queue-min" requirement made        *)
(* concrete.                                                                          *)
(*                                                                                    *)
(* WHY NO EPOCH FRESHNESS GATE OR TWO-PHASE COMMITMENT IS NEEDED (warm-GlobalMin scope). *)
(* In the WARM-CACHE convention (caches AND acks seeded at warmup — see the Init           *)
(* `GlobalMin` ackFloor seed), a pessimistic floor is monotone-NON-DECREASING: it equals   *)
(* the min over the source's folds, so it only ever RISES — and rises ONLY when the source  *)
(* PRUNES a departed origin (losing that origin's low), after which the source ALSO         *)
(* freezes/confirms high (it no longer folds the low). So a snapshot ack is either current  *)
(* or stale-LOW (the source has since pruned and risen); a stale-LOW ack makes the observer *)
(* HOLD lower than necessary (liveness, healed by WF re-ack), never advance too far         *)
(* (safety). CAVEAT — this monotone-rise argument relies on the warmup having delivered a   *)
(* pessimistic ack (<= GlobalMin) to every observer (the seed) PLUS FreezeNeverBelowGlobalMin: *)
(* an observer's stale ack is then a permanently-valid lower bound. Reverting the seed to     *)
(* the own-receipt value (a NON-pessimistic warmup) reproduces the residual — the seed is     *)
(* load-bearing, so this is NOT "the report alone." The fresh-ack ROUND is modeled by         *)
(* (i) advancing only when every folded non-pruned peer is REACHABLE (the round can complete) *)
(* and (ii) WF SendAck refreshing the snapshots; NO partition-hold (the observer itself holds *)
(* when it cannot reach a folded peer — the lowering side never assumes the observer holds,   *)
(* which the S47 link-heal race proved unsound). The S47-ladder finding sharpened: once the   *)
(* report is pessimistic AND warmup-propagated, AsyncAckGossip's passive epoch GATE is moot   *)
(* (the report VALUE, not the freshness gate, was the broken part) — so the sound consumer     *)
(* needs the pessimistic report (on a fresh-ack round), NOT an epoch comparison. The cold-cache *)
(* corner (an observer that never received a pessimistic ack) is where the epoch still binds.   *)
(***************************************************************************)
\* The pessimistic queue-min a survivor s reports in a fresh ack (and uses for its own
\* local term): the min over its own floor and every non-pruned folded source's cache.
PessimisticFloor(s) ==
    LET ownFloor == IF localDisc[s] THEN localFrame[s] ELSE recvThrough[s]
        folded   == { o \in SURVIVORS \ {s} : ~pruned[s][o] }
    IN IF folded = {} THEN ownFloor
       ELSE Min2(ownFloor, MinI({cacheLast[s][o] : o \in folded}))

\* What a survivor deposits in a fresh ack (SendAck): the PessimisticReport modes (AsyncAckSound
\* and the S49 AsyncAckSoundFresh) report their PESSIMISTIC queue-min; every other async mode
\* reports its own (announce/freeze/receipt) PeerFloor — exactly the non-pessimistic report whose
\* staleness the departed-low + gossip races exploit. (Evaluates to PeerFloor in every
\* non-PessimisticReport mode, so their SendAck is state-identical.)
AckReportFloor(s) == IF PessimisticReport THEN PessimisticFloor(s) ELSE PeerFloor(s)

\* The floor s may advance to: the min over its OWN current pessimistic floor and every
\* folded non-pruned reachable peer's last (pessimistic) ack snapshot. Uses the in-flight
\* ackFloor snapshot (discharging idealization (a)), NOT a live read of the peer's floor.
AsyncSoundAckedFloor(s) ==
    MinI({PessimisticFloor(s)} \union {ackFloor[s][o] : o \in AsyncReachableFolded(s)})

AsyncSoundTarget(s) ==
    \* No-partition-hold: cannot complete a fresh round with an unreachable folded peer -> HOLD.
    IF ~AsyncAllFoldedReachable(s) THEN CurBound(s)
    \* Mesh-agreed AND the local freeze has fully converged to the acked floor: the slot is
    \* excluded and every higher frame is the (final) frozen value, so advance to MAX.
    ELSE IF SlotMeshAgreed(s) /\ localFrame[s] = AsyncSoundAckedFloor(s) THEN MAX_FRAME
    \* Otherwise cap by the pessimistic acked floor — never lock past a frame a folded peer's
    \* pessimistic report says it (or a departed origin it still folds) may still freeze below.
    ELSE Min2(BaselineTarget(s), AsyncSoundAckedFloor(s))

(***************************************************************************)
(* S49 — the COLD-CACHE-SOUND target (AsyncAckSoundFresh). Identical to AsyncSoundTarget  *)
(* EXCEPT for one decisive added gate: an UNRECEIVED-ACK HOLD. The cold residual is that   *)
(* the observer TRUSTS its cold ackFloor seed (the relay's own high receipt) and advances   *)
(* on it before a genuine pessimistic ack arrives; AsyncAckSoundFresh NULL-seeds the ack    *)
(* (Init) and HOLDS for any folded reachable peer whose ack is still NULL (not yet           *)
(* delivered). It therefore advances only on RECEIVED acks, each carrying the source's        *)
(* PESSIMISTIC floor (PessimisticReport) — so the relay that still folds the departed origin   *)
(* delivers the origin's low, pinning the observer at the global min. This is S47's            *)
(* "fresh-ack ROUND postdating the observer's intent" made concrete: COMPLETE the round (a      *)
(* received ack), do not read a stale/cold cache. WF SendAck guarantees the held observer        *)
(* eventually receives every reachable folded peer's ack (liveness).                             *)
(***************************************************************************)
AsyncSoundFreshTarget(s) ==
    \* No-partition-hold: cannot complete a fresh round with an unreachable folded peer -> HOLD.
    IF ~AsyncAllFoldedReachable(s) THEN CurBound(s)
    \* Unreceived-ack hold (the decisive cold delta): do NOT trust the cold seed. HOLD until
    \* every folded reachable peer has DELIVERED a genuine ack (ackFloor # NULL).
    ELSE IF \E o \in AsyncReachableFolded(s) : ackFloor[s][o] = NULL_FRAME THEN CurBound(s)
    \* Mesh-agreed AND converged: excluded, advance to MAX.
    ELSE IF SlotMeshAgreed(s) /\ localFrame[s] = AsyncSoundAckedFloor(s) THEN MAX_FRAME
    \* Otherwise cap by the pessimistic acked floor (now all received), as AsyncSoundTarget.
    ELSE Min2(BaselineTarget(s), AsyncSoundAckedFloor(s))

(***************************************************************************)
(* Candidate fix #4 ("InheritedFloor"): the CACHE-ONLY / NO-WIRE / PASSIVE-          *)
(* CORROBORATION variant — the cheapest, most TEMPTING design, MACHINE-DISPROVEN      *)
(* UNSOUND here (run DoubleFailureRelay_InheritedFloor.cfg: `NoConfirmedDivergence`    *)
(* VIOLATED). It is retained as the THIRD documented dead-end (alongside _Baseline,    *)
(* a safety dead-end, and _Tombstone, a liveness dead-end) precisely because it looks   *)
(* so plausible: it needs no wire-format change and no peer-receipt oracle.             *)
(*                                                                                      *)
(* The design reads ONLY the possibly-stale per-endpoint CACHE (`BaselineTarget`) plus  *)
(* two pieces of LOCAL receiver state:                                                  *)
(*   1. ARM (synchronous): the instant a CONNECTED folded source leaves a survivor's     *)
(*      fold (Die), snapshot that source's last cached term into `floor[s]` (folded into  *)
(*      BOTH the confirm target here AND the freeze via `QueueMin`) — "do not forget a     *)
(*      departed low term." This part is fine and race-free.                              *)
(*   2. RELEASE (event-driven): `floor[s]` lifts when EVERY alive folded source has        *)
(*      delivered FRESH `Gossip` (tracked in `corrob`) reporting D CONNECTED above it.      *)
(*                                                                                          *)
(* WHY IT FAILS — the CORROBORATE-THEN-DROP race: a peer can gossip {connected, high}        *)
(* (corroborating "healthy", releasing the observer's floor) and then DETECT THE DROP        *)
(* itself and freeze the slot LOW, with its disconnect gossip still in flight — so the        *)
(* observer, having already released, confirms+LOCKS the slot's real inputs above the         *)
(* eventual freeze before the drop reaches it (the residual, un-fixed). The obstruction is     *)
(* INTRINSIC: a cache LAGS the corroborator's own in-flight drop, so a passive release acts on  *)
(* info already stale at the source. Disproven here for the natural rule and (in the adversarial *)
(* review) for the strongest variants — re-validate-against-current-cache, never-release         *)
(* (= Tombstone liveness), speculative-bound, link-reachability-hold, local generation counter   *)
(* — each falling to the same race or to liveness. This is the evidence that the                  *)
(* production fix needs a FRESH-ACK ROUND (request/response held until the ack postdates the      *)
(* observer's intent to advance) + a per-slot DROP-EPOCH commitment — the MeshAgree direction      *)
(* (the default), whose aggregation this module proves sound under exactly the instantaneous        *)
(* -fresh-ack idealization that ack round must discharge.                                            *)
(***************************************************************************)
InheritedFloorTarget(s) ==
    IF floor[s] = NULL_FRAME THEN BaselineTarget(s)
    ELSE Min2(BaselineTarget(s), floor[s])

BoundTarget(s) ==
    CASE FIX_MODE = "MeshAgree"      -> MeshAgreeTarget(s)
      [] FIX_MODE = "InheritedFloor" -> InheritedFloorTarget(s)
      [] SoundMode                   -> AsyncSoundTarget(s)   \* S48 — checked before AsyncMode
      [] SoundFreshMode              -> AsyncSoundFreshTarget(s)  \* S49 cold-sound — before AsyncMode
      [] AsyncMode                   -> AsyncAckTarget(s)
      [] OTHER                       -> BaselineTarget(s)

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
    /\ floor \in [SURVIVORS -> Frame]
    /\ corrob \in [SURVIVORS -> [SURVIVORS -> BOOLEAN]]
    /\ gone \in [SURVIVORS -> BOOLEAN]
    /\ pruned \in [SURVIVORS -> [SURVIVORS -> BOOLEAN]]
    /\ slotEpoch \in [SURVIVORS -> 0..EPOCH_MAX]
    /\ ackFloor \in [SURVIVORS -> [SURVIVORS -> Frame]]
    /\ ackEpoch \in [SURVIVORS -> [SURVIVORS -> 0..EPOCH_MAX]]
    /\ cacheEpoch \in [SURVIVORS -> [SURVIVORS -> 0..EPOCH_MAX]]
    /\ announced \in [SURVIVORS -> Frame]

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
    /\ floor = [s \in SURVIVORS |-> NULL_FRAME]
    /\ corrob = [o \in SURVIVORS |-> [s \in SURVIVORS |-> FALSE]]
    \* S45 async machinery. Deterministic at Init (single value) so the OTHER four modes,
    \* which never touch these, keep a byte-identical state space (each contributes a
    \* factor of 1). gone mirrors ~alive (all alive => none gone); pruned all FALSE
    \* (nobody timed-out yet); slotEpoch 0 (no drop yet); the acks seeded from the
    \* warmed-up true receipt at epoch 0 (post-warmup every survivor has a fresh ack of
    \* every other's live receipt), and cacheEpoch 0.
    /\ gone = [s \in SURVIVORS |-> FALSE]
    /\ pruned = [o \in SURVIVORS |-> [s \in SURVIVORS |-> FALSE]]
    /\ slotEpoch = [s \in SURVIVORS |-> 0]
    \* The PessimisticReport modes seed each warmup ack with the PESSIMISTIC floor: at warmup
    \* every survivor folds every other (no pruning), so its pessimistic floor is the GlobalMin
    \* (it sees the low origin's receipt). The non-pessimistic modes keep the own-receipt seed
    \* (state-identical). THIS SEED IS LOAD-BEARING (not free): it encodes a completed warmup
    \* round in which every observer obtained a pessimistic ack (<= GlobalMin) BEFORE any
    \* drop/partition. Replacing it with the own-receipt seed reproduces the residual under
    \* AsyncAckSound (LockedRecordMatchesFreeze VIOLATED — a victim advances on a stale-high
    \* warmup ack while c->b is up, so it does not partition-hold), the warm-GlobalMin premise.
    \* S49: COLD_CACHE = TRUE models the cold corner DIRECTLY — it forces the own-receipt-HIGH
    \* seed even for the PessimisticReport modes, so the warm seed no longer closes the
    \* stale-in-flight facet. AsyncAckSound then FAILS cold (the report alone is not enough — it
    \* trusts the cold seed); AsyncAckSoundFresh PASSES cold (it NULL-seeds and never trusts a
    \* cold seed — see below).
    \* S49 AsyncAckSoundFresh: NULL-seed every ack ("no fresh ack received yet"), independent of
    \* COLD_CACHE — the mode's whole premise is that it NEVER trusts a pre-existing/cold seed and
    \* HOLDS until a genuine ack arrives. (The other modes keep the warm/cold value seed.)
    /\ ackFloor = [o \in SURVIVORS |-> [s \in SURVIVORS |->
                     IF SoundFreshMode THEN NULL_FRAME
                     ELSE IF PessimisticReport /\ ~COLD_CACHE THEN GlobalMin
                     ELSE recvThrough[s]]]
    /\ ackEpoch = [o \in SURVIVORS |-> [s \in SURVIVORS |-> 0]]
    /\ cacheEpoch = [o \in SURVIVORS |-> [s \in SURVIVORS |-> 0]]
    /\ announced = [s \in SURVIVORS |-> NULL_FRAME]

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
    /\ ~gone[src]                          \* async: a stopped survivor cannot SEND (no effect in other modes — gone pinned FALSE)
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
          \* InheritedFloor: a FRESH connected-above-floor delivery corroborates that
          \* `src`'s slot is healthy above obs's armed floor (event-driven, post-merge:
          \* a reordered stale connected packet that the merge rejects sets newDisc/newLast
          \* unchanged, so it cannot falsely corroborate a converged-disconnect cache).
          /\ corrob' = IF /\ FIX_MODE = "InheritedFloor"
                          /\ floor[obs] # NULL_FRAME
                          /\ ~newDisc
                          /\ newLast > floor[obs]
                       THEN [corrob EXCEPT ![obs][src] = TRUE]
                       ELSE corrob
          \* Async: gossip also carries src's CURRENT drop-epoch, merged monotone-UP into
          \* obs's epoch tracker (obs learns the latest generation src has reached). This is
          \* the gossip-delivered epoch the AsyncAckTwoPhase freshness gate compares against —
          \* the floor of what the observer knows about src's commitment generation.
          /\ cacheEpoch' = IF AsyncMode
                           THEN [cacheEpoch EXCEPT ![obs][src] = Max2(cacheEpoch[obs][src], slotEpoch[src])]
                           ELSE cacheEpoch
    /\ UNCHANGED <<recvThrough, alive, localDisc, localFrame, link, bound, recSrc, floor,
                   gone, pruned, slotEpoch, ackFloor, ackEpoch, announced>>

(***************************************************************************)
(* Action: src directly concludes the dropped slot disconnected (an explicit    *)
(* remove_player or a disconnect timeout). It freezes at its own current        *)
(* queue-min (its receipt, mined no lower than any folded lower cache). Models   *)
(* remove_player(D) on a survivor.                                              *)
(***************************************************************************)
QueueMin(s) ==
    LET folded   == FoldedSources(s)
        base     == recvThrough[s]
        cacheMin == IF folded = {}
                    THEN base
                    ELSE Min2(base, MinI({cacheLast[s][o] : o \in folded}))
    IN \* InheritedFloor: the inherited floor is a departed CONNECTED source's low term
       \* that the live fold no longer carries; it must mine the FREEZE down too (not just
       \* cap the confirm bound), else a survivor that detects the drop itself freezes
       \* ABOVE the departed origin's term and its discarded frames diverge. With the
       \* floor folded here, the resulting freeze is always <= floor, so the post-freeze
       \* subsume is safe.
       IF FIX_MODE = "InheritedFloor" /\ floor[s] # NULL_FRAME
       THEN Min2(cacheMin, floor[s])
       ELSE cacheMin

\* s's currently-PUBLISHED floor (what its gossip/acks carry), ignoring any pending
\* two-phase announce.
ReportedFloor(s) == IF localDisc[s] THEN localFrame[s] ELSE recvThrough[s]

(***************************************************************************)
(* (AsyncAckTwoPhase two-phase) LowerSafe — s may COMMIT a freeze-lowering only when      *)
(* every observer that still folds s has, for the announced epoch, EITHER already heard  *)
(* the bump (cacheEpoch caught up -> its stale-high ack is now stale -> it HOLDS on the   *)
(* freshness gate) OR is partitioned from s (-> it HOLDS on reachability) OR already       *)
(* holds an ack no higher than the pending low. This is the reverse half of the           *)
(* fresh-ack ROUND: the announce (epoch bump + gossip + published pending-low) must have   *)
(* REACHED every observer before s exposes the actual freeze, so no observer can still     *)
(* lock against a stale-high ack. (Reading cacheEpoch[o][s] directly models the           *)
(* production observer->source REVERSE ACK of the epoch — the commitment handshake the     *)
(* S42 instantaneous-fresh-ack idealization elided; see the header.)                       *)
(***************************************************************************)
\* NOTE the ABSENCE of a `~link[s][o]` (partitioned -> o holds) disjunct: a partition-hold
\* is UNSOUND because partitions HEAL — if s commits its freeze while o is partitioned and
\* the link then heals before o hears the bump, o races on its stale pre-partition ack
\* (the S47 link-heal counterexample). So s must wait until every folding observer has
\* ACTUALLY heard the epoch bump (cacheEpoch caught up -> its stale-high ack is now stale
\* -> it HOLDS on the freshness gate), or already holds an ack no higher than the pending
\* low. A permanently-unreachable observer is instead removed by Prune once it is `gone`.
LowerSafe(s) ==
    \A o \in SURVIVORS \ {s} :
        (~gone[o] /\ ~pruned[o][s]) =>
            \/ cacheEpoch[o][s] >= slotEpoch[s]
            \/ ackFloor[o][s] <= announced[s]

(***************************************************************************)
(* Action (AsyncAckTwoPhase two-phase only): ANNOUNCE a pending freeze-lowering — PHASE 1.  *)
(* s bumps + (via Gossip) propagates its drop-epoch and PUBLISHES the pending-low frame    *)
(* as its reported floor (PeerFloor reads `announced`), WITHOUT yet exposing the freeze in  *)
(* its own confirmed records. A fresh re-ack during the window therefore already carries    *)
(* the low; an un-refreshed observer's stale-high ack goes stale once the bump gossip lands. *)
(* The COMMIT (DetectDrop / lowering UpdateDisconnects, PHASE 2) waits on LowerSafe.        *)
(***************************************************************************)
AnnounceLower(s) ==
    /\ TwoPhase
    /\ ~gone[s]
    /\ announced[s] = NULL_FRAME
    /\ QueueMin(s) < ReportedFloor(s)        \* a genuine lowering is pending
    /\ slotEpoch[s] < EPOCH_MAX              \* bound the per-slot epoch (model finiteness)
    /\ announced' = [announced EXCEPT ![s] = QueueMin(s)]
    /\ slotEpoch' = [slotEpoch EXCEPT ![s] = slotEpoch[s] + 1]
    /\ UNCHANGED <<recvThrough, alive, localDisc, localFrame, cacheDisc, cacheLast,
                   link, bound, recSrc, floor, corrob,
                   gone, pruned, ackFloor, ackEpoch, cacheEpoch>>

DetectDrop(s) ==
    /\ alive[s]
    /\ ~localDisc[s]
    \* AsyncAckTwoPhase two-phase gate: a freeze BELOW the published floor must be
    \* announced (phase 1) and LowerSafe (every observer heard / partitioned) before it
    \* commits; a freeze AT s's own receipt (no lower) commits freely.
    /\ TwoPhase => (QueueMin(s) = recvThrough[s] \/ (announced[s] # NULL_FRAME /\ LowerSafe(s)))
    /\ localDisc' = [localDisc EXCEPT ![s] = TRUE]
    /\ localFrame' = [localFrame EXCEPT ![s] =
           IF TwoPhase /\ announced[s] # NULL_FRAME THEN announced[s] ELSE QueueMin(s)]
    \* InheritedFloor: once s has its own disconnect view of D, the real freeze + frozen
    \* value govern, so the pre-disconnect connected-term floor is obsolete (subsumed).
    /\ floor' = IF FIX_MODE = "InheritedFloor" THEN [floor EXCEPT ![s] = NULL_FRAME] ELSE floor
    /\ corrob' = IF FIX_MODE = "InheritedFloor"
                 THEN [corrob EXCEPT ![s] = [x \in SURVIVORS |-> FALSE]] ELSE corrob
    \* Async (a): this connected->disconnected transition LOWERS s's floor, so it BUMPS s's
    \* per-slot drop-epoch — superseding every ack s has outstanding at the old generation.
    \* In AsyncAckStale/Gossip the bump is here (single-phase). In AsyncAckTwoPhase the bump
    \* already happened in AnnounceLower (phase 1), so it is NOT repeated here. (In the four
    \* original modes slotEpoch stays pinned.)
    /\ slotEpoch' = IF AsyncMode /\ ~TwoPhase
                    THEN [slotEpoch EXCEPT ![s] = Min2(slotEpoch[s] + 1, EPOCH_MAX)]
                    ELSE slotEpoch
    \* AsyncAckTwoPhase: clear the consumed pending-low announcement.
    /\ announced' = IF TwoPhase THEN [announced EXCEPT ![s] = NULL_FRAME] ELSE announced
    /\ UNCHANGED <<recvThrough, alive, cacheDisc, cacheLast, link, bound, recSrc,
                   gone, pruned, ackFloor, ackEpoch, cacheEpoch>>

(***************************************************************************)
(* Action: a survivor dies / is pruned (explicit remove_player by a peer, or a  *)
(* disconnect timeout). Removed from every fold immediately.                    *)
(***************************************************************************)
Die(s) ==
    /\ alive[s]
    /\ Cardinality({o \in SURVIVORS : alive[o]}) > 1   \* never empty the mesh
    /\ alive' = [alive EXCEPT ![s] = FALSE]
    \* InheritedFloor ARM (synchronous with the death — the race-free core): every
    \* surviving observer o that still folded s as a CONNECTED source snapshots s's
    \* last cached term into floor[o], so the bound cannot jump up when s leaves the
    \* fold. `~cacheDisc[o][s]` (s connected in o's cache) implies o is not yet
    \* mesh-agreed, so this only arms while the slot is still contested.
    \* Arm only for observers that have NOT yet detected the drop themselves
    \* (`~localDisc[o]`): one that already has its own disconnect view froze its slot
    \* while s was still folded, so it missed no term and an armed floor would only
    \* (wrongly) pin its confirmation below the living floor forever.
    /\ floor' = IF FIX_MODE = "InheritedFloor"
                THEN [o \in SURVIVORS |->
                       IF o # s /\ alive[o] /\ ~localDisc[o] /\ ~cacheDisc[o][s]
                       THEN IF floor[o] = NULL_FRAME
                            THEN cacheLast[o][s]
                            ELSE Min2(floor[o], cacheLast[o][s])
                       ELSE floor[o]]
                ELSE floor
    \* A (re)arm to a fresh-or-strictly-lower floor resets corroboration: the
    \* remaining alive peers must freshly re-corroborate against the new floor.
    /\ corrob' = IF FIX_MODE = "InheritedFloor"
                 THEN [o \in SURVIVORS |->
                        IF /\ o # s /\ alive[o] /\ ~localDisc[o] /\ ~cacheDisc[o][s]
                           /\ (floor[o] = NULL_FRAME \/ cacheLast[o][s] < floor[o])
                        THEN [x \in SURVIVORS |-> FALSE]
                        ELSE corrob[o]]
                 ELSE corrob
    \* Async (b): the death is the GROUND TRUTH stop (`gone`); unlike the synchronized
    \* `alive` prune of the original modes, observers learn it at SKEWED times via the
    \* separate per-observer `Prune` action below. (In the original modes `gone` stays
    \* pinned FALSE — nothing reads it there, so they are byte-identical.)
    /\ gone' = IF AsyncMode THEN [gone EXCEPT ![s] = TRUE] ELSE gone
    /\ UNCHANGED <<recvThrough, localDisc, localFrame, cacheDisc, cacheLast,
                   link, bound, recSrc, pruned, slotEpoch, ackFloor, ackEpoch, cacheEpoch,
                   announced>>

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
                   cacheLast, bound, recSrc, floor, corrob, asyncVars>>

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
           rawFrame == IF localDisc[s] THEN Min2(localFrame[s], qmin) ELSE qmin
           \* AsyncAckTwoPhase commits to the ANNOUNCED pending-low (phase 2); otherwise the
           \* freshly-folded value.
           newFrame == IF TwoPhase /\ announced[s] # NULL_FRAME THEN announced[s] ELSE rawFrame
       IN /\ \/ ~localDisc[s]                  \* first adopt
             \/ qmin < localFrame[s]           \* or a strictly-lower converge-down
          \* AsyncAckTwoPhase two-phase gate: a genuine lowering must be announced (phase 1) +
          \* LowerSafe; an adopt at the already-published floor (no value lower) commits free.
          /\ TwoPhase => (rawFrame = ReportedFloor(s)
                          \/ (announced[s] # NULL_FRAME /\ LowerSafe(s)))
          /\ localDisc' = [localDisc EXCEPT ![s] = newDisc]
          /\ localFrame' = [localFrame EXCEPT ![s] = newFrame]
          /\ recSrc' = [recSrc EXCEPT ![s] =
                 [g \in 0..MAX_FRAME |->
                     IF /\ recSrc[s][g] # NULL_FRAME      \* committed
                        /\ g > newFrame                   \* above the new freeze
                        /\ ~Locked(s, g)                  \* still re-rollable (not discarded)
                     THEN newFrame                        \* re-roll to frozen value
                     ELSE recSrc[s][g]]]
          \* InheritedFloor SUBSUME: once s has folded a disconnect and converged its
          \* own freeze, the real freeze (<= floor, carried by the relay that triggers
          \* this) governs the slot; the connected-term floor is obsolete and lifts so
          \* the slot can later mesh-agree-exclude (avoids capping below the exclusion).
          /\ floor' = IF FIX_MODE = "InheritedFloor"
                      THEN [floor EXCEPT ![s] = NULL_FRAME] ELSE floor
          /\ corrob' = IF FIX_MODE = "InheritedFloor"
                       THEN [corrob EXCEPT ![s] = [x \in SURVIVORS |-> FALSE]] ELSE corrob
          \* Async (a): this fold only fires when it LOWERS s's floor (first-adopt
          \* connected->disconnected, or a strictly-lower converge-down), so it BUMPS
          \* s's drop-epoch. NOTE FOR PRODUCTION: the S46 `arm_status_epoch` bumps only on
          \* connected<->disconnected, NOT on a freeze converge-DOWN; the converge-down bump
          \* (single-phase in Stale/Gossip; in AnnounceLower for two-phase Commit) is a
          \* concrete REQUIREMENT the src/ fix must add (see the header finding).
          /\ slotEpoch' = IF AsyncMode /\ ~TwoPhase
                          THEN [slotEpoch EXCEPT ![s] = Min2(slotEpoch[s] + 1, EPOCH_MAX)]
                          ELSE slotEpoch
          /\ announced' = IF TwoPhase THEN [announced EXCEPT ![s] = NULL_FRAME] ELSE announced
    /\ UNCHANGED <<recvThrough, alive, cacheDisc, cacheLast, link, bound,
                   gone, pruned, ackFloor, ackEpoch, cacheEpoch>>

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
                   cacheLast, link, floor, corrob, asyncVars>>

(***************************************************************************)
(* Action (InheritedFloor mode): RELEASE the inherited floor once EVERY alive       *)
(* folded source has freshly corroborated D connected above it — positive,           *)
(* event-driven proof the departed peer was a benign laggard, so confirmation may     *)
(* resume past the floor. A survivor partitioned from a peer it needs never collects   *)
(* that peer's corroboration, so it cannot release and HOLDS (the production           *)
(* partition-hold); WF on Gossip+Unblock guarantees the corroborations eventually      *)
(* arrive (or the peer dies and leaves the fold), so the hold is bounded.              *)
(***************************************************************************)
ReleaseFloor(s) ==
    /\ FIX_MODE = "InheritedFloor"
    /\ alive[s]
    /\ floor[s] # NULL_FRAME
    /\ \A o \in FoldedSources(s) : corrob[s][o]   \* all alive folded peers corroborated
    /\ floor' = [floor EXCEPT ![s] = NULL_FRAME]
    /\ corrob' = [corrob EXCEPT ![s] = [x \in SURVIVORS |-> FALSE]]
    /\ UNCHANGED <<recvThrough, alive, localDisc, localFrame, cacheDisc, cacheLast,
                   link, bound, recSrc, asyncVars>>

(***************************************************************************)
(* Action (async modes only): PER-OBSERVER PRUNE — obs's disconnect-timeout      *)
(* observation of src's death. Discharges idealization (b): obs removes src from  *)
(* its OWN fold (`pruned[obs][src]`) only after src has actually stopped           *)
(* (`gone[src]`), and DIFFERENT observers fire this at SKEWED times (production's  *)
(* per-endpoint disconnect timeout), so the synchronized global `alive` prune is   *)
(* replaced by a per-observer view. A link-down-but-alive peer is NOT pruned       *)
(* (production keeps it Running until the timeout) — only a genuinely `gone` peer  *)
(* leaves a fold here, which is the death-driven fold-membership asymmetry the      *)
(* residual exploits. WF (below) forces every gone src to eventually be pruned by   *)
(* every obs, so folds converge.                                                   *)
(***************************************************************************)
Prune(obs, src) ==
    /\ AsyncMode
    /\ obs # src
    /\ gone[src]
    /\ ~pruned[obs][src]
    /\ pruned' = [pruned EXCEPT ![obs][src] = TRUE]
    /\ UNCHANGED <<recvThrough, alive, localDisc, localFrame, cacheDisc, cacheLast,
                   link, bound, recSrc, floor, corrob,
                   gone, slotEpoch, ackFloor, ackEpoch, cacheEpoch, announced>>

(***************************************************************************)
(* Action (async modes only): SEND-ACK — src answers obs's fresh floor-request     *)
(* over a live link, depositing src's CURRENT floor and drop-epoch into obs's        *)
(* per-source ack slot. This is the CONCRETE in-flight ack that discharges            *)
(* idealization (a): MeshAgree's `PeerFloor(o)` read src's true CURRENT floor with    *)
(* zero delay; here obs only ever holds the value src last SENT. THE STALENESS: src   *)
(* may afterwards LOWER its floor (DetectDrop / converge-down) and bump its epoch      *)
(* WITHOUT a new SendAck, leaving obs's ack stale-HIGH at an OLD epoch — exactly the   *)
(* in-flight gap. A gone src cannot ack; obs requests only from peers still in its     *)
(* fold (`~pruned[obs][src]`) and reachable. WF forces acks to eventually refresh.     *)
(***************************************************************************)
SendAck(src, obs) ==
    /\ AsyncMode
    /\ src # obs
    /\ ~gone[src]
    /\ ~pruned[obs][src]
    /\ link[src][obs]
    /\ ackFloor' = [ackFloor EXCEPT ![obs][src] = AckReportFloor(src)]
    /\ ackEpoch' = [ackEpoch EXCEPT ![obs][src] = slotEpoch[src]]
    /\ UNCHANGED <<recvThrough, alive, localDisc, localFrame, cacheDisc, cacheLast,
                   link, bound, recSrc, floor, corrob,
                   gone, pruned, slotEpoch, cacheEpoch, announced>>

Next ==
    \/ \E src, obs \in SURVIVORS : Gossip(src, obs)
    \/ \E s \in SURVIVORS : DetectDrop(s)
    \/ \E s \in SURVIVORS : Die(s)
    \/ \E src, dst \in SURVIVORS : Unblock(src, dst)
    \/ \E s \in SURVIVORS : UpdateDisconnects(s)
    \/ \E s \in SURVIVORS : AdvanceConfirm(s)
    \/ \E s \in SURVIVORS : ReleaseFloor(s)
    \* Async-only (guarded by AsyncMode / TwoPhase, so disabled — zero new states — in the
    \* four original modes, keeping them byte-identical).
    \/ \E obs, src \in SURVIVORS : Prune(obs, src)
    \/ \E src, obs \in SURVIVORS : SendAck(src, obs)
    \/ \E s \in SURVIVORS : AnnounceLower(s)

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
    /\ \A s \in SURVIVORS : WF_vars(ReleaseFloor(s))
    /\ WF_vars(\E src, dst \in SURVIVORS : Unblock(src, dst))
    \* Async-only liveness: every gone peer is eventually pruned by every observer
    \* (folds converge), and acks eventually refresh (a held survivor re-requests and
    \* re-hears a current-epoch floor). Vacuously satisfied in the original modes
    \* (Prune/SendAck never enabled there), so the four modes' liveness is unchanged.
    /\ \A obs, src \in SURVIVORS : WF_vars(Prune(obs, src))
    /\ \A src, obs \in SURVIVORS : WF_vars(SendAck(src, obs))
    /\ \A s \in SURVIVORS : WF_vars(AnnounceLower(s))

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
