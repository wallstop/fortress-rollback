----------------------- MODULE SpectatorReactivationEpoch -----------------------

(***************************************************************************)
(* TLA+ Specification for the HOST -> SPECTATOR REACTIVATION-EPOCH gate      *)
(* (Session 46) -- the reordering-aware, MULTI-CYCLE companion to            *)
(* SpectatorFailover.tla (Session 39).                                      *)
(*                                                                         *)
(* WHY A SEPARATE COMPANION, NOT A BUMP OF SpectatorFailover.tla             *)
(*                                                                         *)
(* SpectatorFailover.tla proves the converge-DOWN (F4 / critic-#2) and the   *)
(* provenance-gated reactivation SOUNDNESS against a stale-lagging canonical  *)
(* host, but only under a deliberately STRONGER-than-production scope: ONE    *)
(* drop cycle and IN-ORDER per-host staging. Its own SCOPE header lists the   *)
(* two production fail-OPEN residuals it therefore CANNOT see:               *)
(*                                                                         *)
(*   (ii)  CROSS-CYCLE: a drop-AFTER-a-rejoin whose converged (global-min)    *)
(*         freeze a FIRST cycle's stale, pre-convergence-high witness         *)
(*         numerically covers -- so a reordered earlier-cycle drop report      *)
(*         re-arms consumed provenance and a stale-connected report then       *)
(*         resurrects the SECOND drop.                                        *)
(*   (iii) WITHIN-CYCLE: a genuine current-drop witness's OWN reordered        *)
(*         PRE-drop connected snapshot, first-writer-staged at a later frame,  *)
(*         transiently resurrects the slot until its next drop-bearing packet. *)
(*                                                                         *)
(* Both fail-opens share ONE root cause -- connect-status reports carry no     *)
(* cycle identity, so a freeze-only witness cannot tell a stale earlier-cycle  *)
(* report from a current one. The Session-46 fix adds that identity: a         *)
(* per-slot `ConnectionStatus.epoch` (a u16 generation the owning host bumps   *)
(* on every connected<->disconnected transition), tracked by the spectator as  *)
(* a per-(host,player) high-water (`host_status_epoch`) and stamped onto each  *)
(* witnessed drop (`DropWitness { freeze, epoch }`). This spec DROPS both       *)
(* scope assumptions (multi-cycle generations + reordered staging) so BOTH      *)
(* fail-opens become reachable, and machine-checks that the epoch gates close   *)
(* them. It is the "reordering-aware multi-cycle spec" the audit's tracked      *)
(* follow-up names. SpectatorFailover.tla is left UNCHANGED (it still owns the  *)
(* single-cycle converge-down proof and the liveness property).                *)
(*                                                                         *)
(* WHAT THE PRODUCTION CODE DOES (and what this models)                     *)
(*                                                                         *)
(*  - ConnectionStatus.epoch (messages.rs:37): per-slot u16 generation,       *)
(*    bumped by the OWNING host on every connected<->disconnected transition   *)
(*    (`arm_status_epoch`, p2p_session.rs:7266) -- a drop or a reactivation.    *)
(*    Modeled as the generation index itself: a drop is an ODD generation, a   *)
(*    connect/rejoin an EVEN one, so a report at generation g carries epoch g.  *)
(*  - host_status_epoch[h][p] (p2p_spectator_session.rs:156): monotone        *)
(*    high-water of EVERY epoch seen in host h's stream for slot p. Modeled    *)
(*    as `highWater[h]`.                                                       *)
(*  - host_drop_witness[h][p] = Option<DropWitness{freeze,epoch}>             *)
(*    (:144, :2057): host h's most-recent WITNESSED drop. Modeled as          *)
(*    `witness[h]` (freeze = NULL_FRAME means None).                          *)
(*  - witness_host_status_reports (:1427): advance the high-water by every     *)
(*    report's epoch; record/refresh the witness for a DISCONNECTED report     *)
(*    whose epoch is NOT strictly below the high-water (`is_stale`, :1444) --   *)
(*    newer epoch RESETS, same epoch MAX-merges the freeze. The stale-reject   *)
(*    closes the CROSS-CYCLE fail-open.                                        *)
(*  - reactivation_provenance (:1510): a `disconnected->connected` report may   *)
(*    be FOLLOWED iff the latch is disconnected, the witness freeze `>=` the    *)
(*    latch freeze, AND `incoming_epoch >= witness.epoch` (:1529). The epoch    *)
(*    clause closes the WITHIN-CYCLE fail-open; `>=` (not `>`) keeps the gate   *)
(*    inert for legacy uniform-epoch peers.                                    *)
(*  - merge_connection_status (:2186): the four arms -- (T,T) min-converge;     *)
(*    (T,F) follow-iff-Witnessed; (F,T) adopt; (F,F) max. A follow CONSUMES     *)
(*    every host's witness (:1778 / :1550); an adopt RE-ARMS the committing     *)
(*    host's witness at the adopted freeze+epoch (:1788 / :1576).               *)
(*                                                                         *)
(* THE FIX_MODE CONVENTION (mirrors DoubleFailureRelay.tla)                  *)
(*   - FIX_MODE = "Epoch"        : both Session-46 gates ON. DEFAULT cfg;      *)
(*                                 PASSES every safety property.               *)
(*   - FIX_MODE = "EpochProvOnly": the provenance epoch clause ON, the high-   *)
(*                                 water stale-reject OFF. Also PASSES -- this   *)
(*                                 ISOLATES the provenance clause as the gate   *)
(*                                 binding at this (fixed-canonical) bound, and *)
(*                                 confirms the high-water gate is modeled      *)
(*                                 faithfully but NOT load-bearing here (its    *)
(*                                 cross-cycle re-arm needs failover -- see       *)
(*                                 SCOPE). A manual mode-isolation cfg (like    *)
(*                                 the DoubleFailureRelay variant cfgs, NOT     *)
(*                                 auto-run by verify-tla.sh).                  *)
(*   - FIX_MODE = "EpochBlind"   : the pre-S46 (Session-31/39) freeze-ONLY      *)
(*                                 witness -- BOTH gates off. DEMO-FAIL cfg      *)
(*                                 (SpectatorReactivationEpoch_EpochBlind.cfg,  *)
(*                                 NOT registered in verify-tla.sh): TLC        *)
(*                                 reports NoFalseResurrection VIOLATED,        *)
(*                                 reproducing the WITHIN-CYCLE fail-open --      *)
(*                                 proves the provenance clause is load-bearing.*)
(*                                                                         *)
(* PROPERTIES VERIFIED (FIX_MODE = "Epoch")                                 *)
(*   - NoFalseResurrection (THE headline): the latch never shows CONNECTED      *)
(*     while some host observes a strictly NEWER drop the spectator has already  *)
(*     COMMITTED past (the spectator would replay live values for frames a host  *)
(*     froze -- a spectator-vs-host desync). EpochBlind violates it (a reordered  *)
(*     pre-drop connected snapshot is FOLLOWED, driving the latch backward);     *)
(*     Epoch holds it (the provenance epoch clause refuses the follow). See the  *)
(*     property's own header for the committed-gated / strictly-newer guards.    *)
(*   - LatchAtOrBelowLiveMin (F4 / critic-#2 preserved): a latched-disconnected *)
(*     slot's freeze is <= the global-min freeze across live hosts staging a    *)
(*     drop -- the epoch gates do not regress converge-down.                     *)
(*   - GateAcceptsBoundaryWitness (availability preserved): a genuine           *)
(*     current-generation witness at EXACTLY the freeze boundary IS classified  *)
(*     Witnessed -- pins the load-bearing FREEZE `>=` (not `>`): tightening it    *)
(*     to `>` makes a reachable boundary RED. (The EPOCH `>=`'s equality case   *)
(*     is the LEGACY uniform-epoch world and is unreachable in this generation   *)
(*     model -- a connected report carries an EVEN generation, a drop witness an  *)
(*     ODD one, so `staged.epoch = witness.epoch` never occurs; only the `>`     *)
(*     side of the epoch comparison is exercised here. The epoch `==` boundary   *)
(*     is covered by the production unit test                                    *)
(*     within_cycle_pre_drop_epoch_blocks_follow_post_drop_epoch_follows, which  *)
(*     asserts reactivation_provenance(.,.,1) == Witnessed at witness epoch 1.)  *)
(*   - FreezeNeverRaised (monotone-down preserved): across a step that keeps    *)
(*     the latch disconnected, its freeze frame never rises.                    *)
(*                                                                         *)
(* SCOPE / FAITHFUL SIMPLIFICATIONS                                          *)
(*   - ONE droppable player (the production merge loops players with no         *)
(*     cross-player coupling, :1691) across UP TO TWO drop cycles               *)
(*     (generations 0..MAXGEN = connected, dropped, rejoined, dropped) -- the    *)
(*     minimum that exhibits BOTH fail-opens (within-cycle needs one drop;       *)
(*     cross-cycle needs a drop-rejoin-drop).                                    *)
(*   - Reordered staging is modeled directly: ReceiveReport(h, g) may stage      *)
(*     ANY generation g <= hostObs[h] -- the CURRENT report (g = hostObs[h]) or  *)
(*     a REORDERED earlier one (g < hostObs[h]). This is the in-order-staging     *)
(*     assumption SpectatorFailover.tla makes and this spec deliberately drops.   *)
(*   - Per-host freeze is a fixed-at-Init asymmetric high-water reused for all   *)
(*     of that host's drops (each host's own received-frame ceiling under loss). *)
(*     A host's earlier-cycle drop therefore freezes at a frame >= the later     *)
(*     cycle's converged global-min -- exactly the numerical-cover precondition   *)
(*     the cross-cycle fail-open needs.                                          *)
(*   - LIVENESS is OUT OF SCOPE here (the original SpectatorFailover.tla owns     *)
(*     DropEventuallyLatched at its single-cycle bound). Reordered staging makes  *)
(*     a faithful liveness obligation delicate (a host may deliver only stale     *)
(*     reports for arbitrarily long); the safety BFS is finite and complete, and  *)
(*     the headline result is the resurrection-SAFETY property, so this spec is   *)
(*     safety-only by design (matching the DoubleFailureRelay demo cfgs).         *)
(*   - HOST FAILOVER / canonical OSCILLATION is OUT OF SCOPE here (owned by        *)
(*     SpectatorFailover.tla, single-cycle): all hosts stay present and the        *)
(*     canonical is the fixed Min(HOSTS). This is a DELIBERATE scope boundary, but  *)
(*     -- to be precise -- NOT because failover is impossible or its corners        *)
(*     "forbidden": adding a Failover action to THIS latch-only model DOES surface  *)
(*     an Epoch-mode NoFalseResurrection counterexample (a CROSS-CYCLE shape: a     *)
(*     second host that in-order-witnessed an EARLIER drop and rejoin becomes       *)
(*     canonical after the first host -- holding a LATER drop -- leaves, and its     *)
(*     stale connected report follows). The reason it is deferred rather than       *)
(*     claimed as a defect is that THIS abstraction cannot ADJUDICATE it: the model *)
(*     collapses a host's per-frame COMMITTED INPUT VALUES (which come from the      *)
(*     canonical's frame-indexed snapshots and DO track the live survivor) and the  *)
(*     latch connect-STATUS (`host_connect_status`, which can transiently lag and   *)
(*     self-corrects on the survivor's next drop-bearing gossip) into a single      *)
(*     `latch` variable. The latch-only over-approximation flags the status lag as  *)
(*     a "resurrection"; whether it is a genuine committed-STATE desync or a        *)
(*     self-correcting status transient depends on the per-frame committed-input    *)
(*     vs latch-status distinction this model omits. (NB the in-scope WITHIN-CYCLE  *)
(*     corner is reachable for the SAME reason the cross-cycle one is -- production  *)
(*     connect-status is SEND-TIME first-writer-wins gossip, NOT frame-monotone in  *)
(*     generation -- so the two are not asymmetric; only their ADJUDICABILITY here  *)
(*     differs, the within-cycle one needing no failover/second-host distinction.)  *)
(*     Faithfully deciding the failover case needs either a frame-buffer-level spec *)
(*     (the per-frame committed-input/staging machinery SpectatorSession.tla        *)
(*     abstracts) or a dedicated src/ arbitration; it is recorded as future work.   *)
(*     The CROSS-CYCLE high-water stale-reject gate that guards the out-of-order    *)
(*     variant is exercised at the gate/unit level by the production test           *)
(*     cross_cycle_stale_drop_epoch_blocks_resurrect_after_consume. With the        *)
(*     canonical fixed here, the WITHIN-CYCLE fail-open reproduces via the          *)
(*     canonical's OWN reordered reports, and the redundant host still drives F4    *)
(*     converge-DOWN (LatchAtOrBelowLiveMin) and the witness table.                *)
(***************************************************************************)
EXTENDS Naturals, FiniteSets, TLC

CONSTANTS HOSTS,        \* Ordered set of redundant hosts; canonical = min live.
          MAX_FRAME,    \* Maximum freeze frame value for model checking.
          NULL_FRAME,   \* Sentinel for "no witness" (-1 in impl).
          MAXGEN,       \* Maximum generation index (drops are odd, connects even).
          FIX_MODE      \* "Epoch" (S46 gates ON) | "EpochBlind" (pre-S46 freeze-only).

ASSUME HOSTS \subseteq Nat /\ HOSTS # {}
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME NULL_FRAME \notin 0 .. MAX_FRAME
ASSUME MAXGEN \in Nat /\ MAXGEN >= 1
ASSUME FIX_MODE \in { "Epoch", "EpochProvOnly", "EpochBlind" }

(* The Session-46 fix has TWO gates. This spec lets each be toggled so their      *)
(* contributions are isolated (mirroring DoubleFailureRelay.tla's FIX_MODE        *)
(* ladder):                                                                       *)
(*   - the PROVENANCE epoch clause in reactivation_provenance                     *)
(*     (`incoming_epoch >= witness.epoch`, :1529) -- closes the WITHIN-CYCLE        *)
(*     fail-open. LOAD-BEARING at this bound: removing it (EpochBlind) makes        *)
(*     NoFalseResurrection RED.                                                    *)
(*   - the HIGH-WATER stale-reject in witness_host_status_reports                  *)
(*     (`is_stale = epoch < high_water`, :1444) -- closes the CROSS-CYCLE re-arm.   *)
(*     Modeled FAITHFULLY but NOT load-bearing at this fixed-canonical bound: the  *)
(*     re-arm-from-CONSUMED it guards needs host FAILOVER (a consumed witness on   *)
(*     a non-canonical host that then becomes canonical) -- out of scope here (see  *)
(*     SCOPE). EpochProvOnly turns it OFF and still PASSES, confirming it is not   *)
(*     binding at this bound (the same "modeled-but-not-pinned-at-this-bound"      *)
(*     honesty SpectatorFailover.tla applies to its consume/re-arm machinery). The *)
(*     cross-cycle gate is exercised at the gate/unit level by the production test *)
(*     cross_cycle_stale_drop_epoch_blocks_resurrect_after_consume.                *)
ProvClauseOn == FIX_MODE \in { "Epoch", "EpochProvOnly" }
HighWaterOn  == FIX_MODE = "Epoch"

(***************************************************************************)
(* Frame values. A drop freeze frame is a real frame 1..MAX_FRAME; a         *)
(* connected slot's last_frame is the baseline 0 (irrelevant to every         *)
(* property, all of which guard on `disconnected`).                          *)
(***************************************************************************)
Frame == { NULL_FRAME } \union ( 0 .. MAX_FRAME )
FreezeFrame == 1 .. MAX_FRAME
CONNECTED_FRAME == 0

GenRange == 0 .. MAXGEN
(* A generation is "disconnected" (a drop) iff odd; "connected" iff even.     *)
GenDisc(g) == g % 2 = 1

(***************************************************************************)
(* A per-player connection status, mirroring the production                  *)
(* `ConnectionStatus { disconnected, last_frame, epoch }`.                   *)
(***************************************************************************)
Status == [ disconnected:BOOLEAN, lastFrame:Frame, epoch:GenRange ]
Connected == [ disconnected |-> FALSE, lastFrame |-> CONNECTED_FRAME, epoch |-> 0 ]

(* A per-host witness slot (host_drop_witness): freeze = NULL_FRAME means None.*)
Witness == [ freeze:Frame, epoch:GenRange ]
NoWitness == [ freeze |-> NULL_FRAME, epoch |-> 0 ]
HasWitness(w) == w.freeze # NULL_FRAME

Min2(x, y) == IF x <= y THEN x ELSE y
Max2(x, y) == IF x >= y THEN x ELSE y
Min(S) == CHOOSE x \in S: \A y \in S: x <= y

(***************************************************************************)
(* Variables                                                               *)
(***************************************************************************)
VARIABLES
  truGen,        \* The mesh's actual generation (monotone up, 0..MAXGEN).
  hostObs,       \* [HOSTS -> GenRange]: host h's observed generation (<= truGen).
  hostFreeze,    \* [HOSTS -> FreezeFrame]: host h's drop freeze ceiling (fixed at Init).
  \* (All hosts stay present: host FAILOVER/removal is OUT OF SCOPE in this
  \*  companion -- it is owned by SpectatorFailover.tla (single-cycle). See the
  \*  SCOPE header for why the JOINT failover + reorder + multi-cycle space needs
  \*  a frame-buffer model the companion deliberately abstracts away. The
  \*  canonical host is therefore the fixed Min(HOSTS).)
  \* --- spectator state (mirrors the production fields) ---
  latch,         \* Status: host_connect_status[player] -- the merged latched view.
  highWater,     \* [HOSTS -> GenRange]: host_status_epoch[h][player].
  witness,       \* [HOSTS -> Witness]: host_drop_witness[h][player].
  staged,        \* [HOSTS -> Status]: latest STAGED status per host (reorderable).
  specGen,       \* GenRange: high-water of generations the spectator has COMMITTED
                 \*   (the latch's `last_recv_frame` mapped to a generation -- commits
                 \*    are frame-ordered, so the committed generation is a monotone
                 \*    high-water and a forward SKIP is impossible).
  \* --- ghost (history) variable, for stating properties only ---
  latchEverDisc  \* TRUE once the latch has ever been disconnected.
                 \* (The generation the latch currently reflects is latch.epoch,
                 \*  carried in the Status record, so no separate ghost is needed.)

vars == << truGen, hostObs, hostFreeze,
           latch, highWater, witness, staged, specGen,
           latchEverDisc >>

(***************************************************************************)
(* The status host h gossips at generation g (its own, possibly reordered,    *)
(* view): an odd generation freezes the slot at h's freeze ceiling; an even    *)
(* generation reports connected at the baseline frame. The epoch is g.        *)
(***************************************************************************)
ReportAt(h, g) ==
  IF GenDisc(g)
  THEN [ disconnected |-> TRUE,  lastFrame |-> hostFreeze[h], epoch |-> g ]
  ELSE [ disconnected |-> FALSE, lastFrame |-> CONNECTED_FRAME, epoch |-> g ]

Canonical == Min(HOSTS)

(***************************************************************************)
(* Type Invariant                                                          *)
(***************************************************************************)
TypeInvariant ==
  /\ truGen \in GenRange
  /\ hostObs \in [HOSTS -> GenRange]
  /\ hostFreeze \in [HOSTS -> FreezeFrame]
  /\ latch \in Status
  /\ highWater \in [HOSTS -> GenRange]
  /\ witness \in [HOSTS -> Witness]
  /\ staged \in [HOSTS -> Status]
  /\ specGen \in GenRange
  /\ latchEverDisc \in BOOLEAN

(***************************************************************************)
(* Initial State: player connected everywhere, nothing staged/witnessed,      *)
(* all hosts live, the latch connected at generation 0. hostFreeze is an       *)
(* adversarial fixed-at-Init asymmetric choice.                               *)
(***************************************************************************)
Init ==
  /\ truGen = 0
  /\ hostObs = [h \in HOSTS |-> 0]
  /\ hostFreeze \in [HOSTS -> FreezeFrame]
  /\ latch = Connected
  /\ highWater = [h \in HOSTS |-> 0]
  /\ witness = [h \in HOSTS |-> NoWitness]
  /\ staged = [h \in HOSTS |-> Connected]
  /\ specGen = 0
  /\ latchEverDisc = FALSE

(***************************************************************************)
(* ENVIRONMENT actions                                                      *)
(***************************************************************************)
(* The mesh ground truth advances one generation (a drop or a genuine rejoin).*)
AdvanceTruGen ==
  /\ truGen < MAXGEN
  /\ truGen' = truGen + 1
  /\ UNCHANGED << hostObs, hostFreeze,
                  latch, highWater, witness, staged, specGen,
                  latchEverDisc >>

(* A host observes the next true transition in its own stream (in-order, never  *)
(* past the ground truth). The spectator learns of it only via a subsequent      *)
(* ReceiveReport -- which may deliver THIS or an EARLIER generation.             *)
AdvanceHostObs(h) ==
  /\ h \in HOSTS
  /\ hostObs[h] < truGen
  /\ hostObs' = [hostObs EXCEPT ![h] = hostObs[h] + 1]
  /\ UNCHANGED << truGen, hostFreeze,
                  latch, highWater, witness, staged, specGen,
                  latchEverDisc >>

(***************************************************************************)
(* SPECTATOR actions                                                        *)
(***************************************************************************)
(* ReceiveReport(h, g): the spectator receives host h's gossip at generation   *)
(* g (g = hostObs[h] is in-order; g < hostObs[h] is a REORDERED earlier         *)
(* packet). Faithful ordering (handle_host_input, :1812):                      *)
(*   1. witness_host_status_reports (:1851) BEFORE staging.                     *)
(*   2. stage the report (host_snapshots; here staged[h]).                     *)
(*   3. converge_latched_drop_status (:1827): lower an already-disconnected     *)
(*      latch whose freeze the report undercuts (never raise).                  *)
(*                                                                         *)
(* Witness update per FIX_MODE: "Epoch" advances the high-water and records a   *)
(* DISCONNECTED report only when NOT strictly below the pre-update high-water   *)
(* (`is_stale`), with newer-resets / same-max-merges. "EpochBlind" is the       *)
(* pre-S46 freeze-only witness: it max-merges the freeze on every disconnected  *)
(* report (no stale-reject) and stamps the epoch but never consults it.         *)
ReceiveReport(h, g) ==
  /\ h \in HOSTS
  /\ g <= hostObs[h]
  /\ LET report == ReportAt(h, g)
         isStale == HighWaterOn /\ report.epoch < highWater[h]
         oldW == witness[h]
         newW ==
           IF report.disconnected /\ ~isStale
           THEN IF HasWitness(oldW)
                THEN IF report.epoch > oldW.epoch
                     THEN [ freeze |-> report.lastFrame, epoch |-> report.epoch ]
                     ELSE IF report.epoch = oldW.epoch
                          THEN [ freeze |-> Max2(oldW.freeze, report.lastFrame),
                                 epoch |-> oldW.epoch ]
                          ELSE oldW   \* strictly older than the witness: never regress
                ELSE [ freeze |-> report.lastFrame, epoch |-> report.epoch ]
           ELSE oldW
     IN /\ highWater' = [highWater EXCEPT ![h] = Max2(highWater[h], report.epoch)]
        /\ witness' = [witness EXCEPT ![h] = newW]
        /\ staged' = [staged EXCEPT ![h] = report]
        /\ latch' =
             IF report.disconnected /\ latch.disconnected /\
                report.lastFrame < latch.lastFrame
             THEN [latch EXCEPT !.lastFrame = report.lastFrame]
             ELSE latch
  /\ UNCHANGED << truGen, hostObs, hostFreeze, specGen, latchEverDisc >>

(* CommitCanonical: commit the canonical host's STAGED status into the latch    *)
(* (commit_canonical_snapshot, :1686). converged_drop_status (:1648) folds a    *)
(* disconnected canonical's freeze DOWN by min over every OTHER live host's      *)
(* staged disconnected freeze; the epoch is the staged report's epoch.          *)
ConvergedFreeze(c) ==
  LET others == { o \in HOSTS \ { c }: staged[o].disconnected }
  IN IF others = {}
     THEN staged[c].lastFrame
     ELSE Min({ staged[c].lastFrame } \union
              { staged[o].lastFrame: o \in others })

(***************************************************************************)
(* The provenance gate (reactivation_provenance, :1510). Witnessed iff the     *)
(* latch is disconnected, c's witness freeze >= the latch freeze, AND -- under   *)
(* "Epoch" -- the incoming connected report's epoch >= the witnessed drop's      *)
(* epoch (:1529). "EpochBlind" drops the epoch clause (freeze-only, pre-S46).   *)
(***************************************************************************)
Witnessed(c, incomingEpoch) ==
  /\ latch.disconnected
  /\ HasWitness(witness[c])
  /\ witness[c].freeze >= latch.lastFrame
  /\ ( ProvClauseOn => incomingEpoch >= witness[c].epoch )

(* Re-arm the committing host's witness at the adopted freeze+epoch              *)
(* (witness_adopted_drop, :1576): newer-resets / same-max-merges.               *)
RearmAdopt(c, fr, ep) ==
  LET oldW == witness[c]
  IN IF HasWitness(oldW)
     THEN IF ep > oldW.epoch
          THEN [ freeze |-> fr, epoch |-> ep ]
          ELSE IF ep = oldW.epoch
               THEN [ freeze |-> Max2(oldW.freeze, fr), epoch |-> oldW.epoch ]
               ELSE oldW
     ELSE [ freeze |-> fr, epoch |-> ep ]

CommitCanonical ==
  /\ LET c == Canonical
         gIn == staged[c].epoch
         incoming ==
           IF staged[c].disconnected
           THEN [ disconnected |-> TRUE,
                  lastFrame |-> ConvergedFreeze(c),
                  epoch |-> staged[c].epoch ]
           ELSE staged[c]
         witnessed == Witnessed(c, staged[c].epoch)
     IN \* Frame-ordered commit: the canonical's committed generation can ADVANCE
        \* the cursor by at most one (the next true generation) or apply an OLDER
        \* reordered-staged status (gIn <= specGen). A forward SKIP is impossible
        \* (the spectator would have to commit an intervening frame first), so the
        \* free-commit model is constrained to the production frame-ordering here.
        \* This is a FAITHFULNESS constraint, not a load-bearing proof element:
        \* removing it leaves every property GREEN at this bound (it only prunes
        \* free-commit states production's frame ordering never reaches).
        /\ gIn <= specGen + 1
        /\ specGen' = Max2(specGen, gIn)
        /\ \/ \* (T,T): both disconnected -> converge down (min). NoTransition.
              /\ latch.disconnected /\ incoming.disconnected
              /\ latch' = [latch EXCEPT !.lastFrame = Min2(latch.lastFrame, incoming.lastFrame)]
              /\ UNCHANGED witness
           \/ \* (T,F): latch disconnected, incoming connected.
              /\ latch.disconnected /\ ~incoming.disconnected
              /\ \/ \* Witnessed -> FOLLOW the genuine reactivation; consume witnesses.
                    /\ witnessed
                    /\ latch' = incoming
                    /\ witness' = [h \in HOSTS |-> NoWitness]
                 \/ \* Unwitnessed -> keep the frozen label (the critic-#1 / S46 gate).
                    /\ ~witnessed
                    /\ UNCHANGED << latch, witness >>
           \/ \* (F,T): adopt a fresh drop; re-arm the committing host's witness.
              /\ ~latch.disconnected /\ incoming.disconnected
              /\ latch' = incoming
              /\ witness' = [witness EXCEPT ![c] = RearmAdopt(c, incoming.lastFrame, incoming.epoch)]
           \/ \* (F,F): both connected -> monotone-up bookkeeping (max). NoTransition.
              /\ ~latch.disconnected /\ ~incoming.disconnected
              /\ latch' = [latch EXCEPT !.lastFrame = Max2(latch.lastFrame, incoming.lastFrame)]
              /\ UNCHANGED witness
  /\ latchEverDisc' = ( latchEverDisc \/ latch'.disconnected )
  /\ UNCHANGED << truGen, hostObs, hostFreeze, highWater, staged >>

(***************************************************************************)
(* Next-state relation                                                      *)
(***************************************************************************)
Next ==
  \/ AdvanceTruGen
  \/ \E h \in HOSTS: AdvanceHostObs(h)
  \/ \E h \in HOSTS, g \in GenRange: ReceiveReport(h, g)
  \/ CommitCanonical

Spec == Init /\ [][Next]_vars

(***************************************************************************)
(* SAFETY PROPERTIES                                                        *)
(***************************************************************************)

(***************************************************************************)
(* NoFalseResurrection (THE headline -- audit critic-#1 / Session 46). A FALSE   *)
(* resurrection is the latch showing CONNECTED (`latch.epoch` = the generation   *)
(* it reflects) while SOME host observes a strictly NEWER drop that the spectator *)
(* has ALREADY committed past -- i.e. the spectator would replay live values for   *)
(* frames a host has frozen (a spectator-vs-host desync). Two guards make this    *)
(* precise and faithful (excluding the benign / liveness states that are NOT      *)
(* desyncs):                                                                       *)
(*                                                                         *)
(*   - committed-gated (`specGen >= hostObs[h]`): a drop a host observed but the  *)
(*     spectator has not yet committed is a LIVENESS gap (it will be adopted on    *)
(*     the next commit), not a safety desync -- the original SpectatorFailover.tla  *)
(*     DropEventuallyLatched owns that. Only a drop the spectator already          *)
(*     processed (committed past) and then UN-did is a resurrection.               *)
(*   - strictly-newer (`hostObs[h] > latch.epoch`): a host lagging at an OLDER     *)
(*     drop than the latch reflects is fine (the latch is ahead).                  *)
(*                                                                         *)
(* (Failover is out of scope here -- all hosts stay present -- so `\A h \in HOSTS`  *)
(* is the full host set; the dead-host exclusion the F4 LatchAtOrBelowLiveMin      *)
(* property makes is SpectatorFailover.tla's concern. See the SCOPE header.)       *)
(*                                                                         *)
(* EpochBlind reproduces it: a reordered PRE-drop connected snapshot (within-     *)
(* cycle), or a re-armed stale witness authorizing a stale connected report       *)
(* (cross-cycle), FOLLOWS a connected report and drives `latch.epoch` BACKWARD    *)
(* below a host's already-committed drop. Epoch holds it: the stale-reject (high- *)
(* water) and the `incoming_epoch >= witness.epoch` gate refuse the follow.       *)
(***************************************************************************)
NoFalseResurrection ==
  ( latchEverDisc /\ ~latch.disconnected ) =>
    \A h \in HOSTS:
      ~( /\ GenDisc(hostObs[h])
         /\ hostObs[h] > latch.epoch
         /\ specGen >= hostObs[h] )

(***************************************************************************)
(* LatchAtOrBelowLiveMin (F4 / completeness-critic #2 -- preserved under the     *)
(* epoch gates). A latched-disconnected slot's freeze is never above the global- *)
(* min freeze across live hosts that staged a drop.                             *)
(***************************************************************************)
LiveDroppedStaged == { h \in HOSTS: staged[h].disconnected }
LiveMinFreeze == Min({ staged[h].lastFrame: h \in LiveDroppedStaged })

LatchAtOrBelowLiveMin ==
  ( latch.disconnected /\ LiveDroppedStaged # {} ) =>
    latch.lastFrame <= LiveMinFreeze

(***************************************************************************)
(* GateAcceptsBoundaryWitness (availability -- preserved). A genuine current-    *)
(* generation witness whose freeze sits at EXACTLY the converged latch freeze    *)
(* AND whose incoming connected report is at or past the witnessed drop's epoch   *)
(* MUST be classified Witnessed -- so its genuine reactivation is FOLLOWED, not    *)
(* wrongly frozen out. Pins the load-bearing FREEZE `>=` (not `>`) in            *)
(* reactivation_provenance (:1528): tightening the freeze comparison to `>` makes *)
(* a real-and-reachable boundary state RED. The EPOCH `>=` (:1529) reduces to    *)
(* `>` AT THIS BOUND -- a connected staged report carries an EVEN generation and  *)
(* a drop witness an ODD one, so `staged.epoch = witness.epoch` is unreachable    *)
(* (the legacy uniform-epoch `==` case lives outside this generation model and is *)
(* covered by the production unit test                                           *)
(* within_cycle_pre_drop_epoch_blocks_follow_post_drop_epoch_follows). So only    *)
(* the FREEZE `>=` is mutation-pinned here; tightening the epoch `>=` to `>`      *)
(* leaves this GREEN (its `=` boundary is never exercised).                      *)
(***************************************************************************)
GateAcceptsBoundaryWitness ==
  LET c == Canonical
  IN ( /\ latch.disconnected
       /\ ~staged[c].disconnected
       /\ HasWitness(witness[c])
       /\ witness[c].freeze = latch.lastFrame
       /\ staged[c].epoch >= witness[c].epoch ) => Witnessed(c, staged[c].epoch)

SafetyInvariant ==
  /\ TypeInvariant
  /\ NoFalseResurrection
  /\ LatchAtOrBelowLiveMin
  /\ GateAcceptsBoundaryWitness

(***************************************************************************)
(* FreezeNeverRaised: across a step that keeps the latch disconnected, its      *)
(* freeze frame is non-increasing (the min / converge arms only lower it).      *)
(***************************************************************************)
FreezeNeverRaised ==
  [][ ( latch.disconnected /\ latch'.disconnected ) =>
        latch'.lastFrame <= latch.lastFrame ]_vars

(***************************************************************************)
(* State constraint: present for parity / future bound tuning (all variables   *)
(* already finite).                                                            *)
(***************************************************************************)
StateConstraint == TRUE

(***************************************************************************)
(* Theorems                                                                *)
(***************************************************************************)
THEOREM SafetySpec == Spec => []SafetyInvariant
THEOREM FreezeSpec == Spec => FreezeNeverRaised

=============================================================================
