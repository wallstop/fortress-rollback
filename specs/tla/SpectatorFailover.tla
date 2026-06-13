-------------------------- MODULE SpectatorFailover --------------------------

(***************************************************************************)
(* TLA+ Specification for MULTI-HOST SPECTATOR FAILOVER                     *)
(* (the companion to SpectatorSession.tla that the N-player desync audit    *)
(*  flagged as the one structural re-model still outstanding).             *)
(*                                                                         *)
(* WHY THIS IS A COMPANION, NOT A BUMP OF SpectatorSession.tla              *)
(*                                                                         *)
(* SpectatorSession.tla models a SINGLE spectator watching a SINGLE host:   *)
(* sync handshake, the circular input buffer, catchup, and frame advance.   *)
(* Its own "Simplifications" header states multi-host is not modeled. The   *)
(* production spectator (src/sessions/p2p_spectator_session.rs) instead      *)
(* receives input broadcasts from a VECTOR of redundant hosts and FAILS     *)
(* OVER when the canonical host disconnects. That failover path carries the  *)
(* desync-prevention machinery this module formalizes -- a structural        *)
(* re-model (a new state shape: per-host views, a per-(host,player)          *)
(* disconnect-witness table, a latched merged view), not a constant bump,    *)
(* which is why it lives in its own file (mirroring FreezeConvergence.tla /   *)
(* FrameAdvantageAggregation.tla, the S37/S38 companions).                  *)
(*                                                                         *)
(* WHAT THE PRODUCTION CODE DOES (and what this models)                     *)
(*                                                                         *)
(* The spectator keeps ONE latched view per player,                         *)
(* `host_connect_status[p] = (disconnected, last_frame)`, merged across all  *)
(* redundant hosts. Each host gossips its OWN view of every player's status  *)
(* (gossip rides every input packet). Under asymmetric packet loss the hosts *)
(* disagree on WHEN/WHETHER a peer dropped and on the freeze frame -- the     *)
(* whole reason redundant hosts exist. The merge                            *)
(* (`merge_connection_status`, p2p_spectator_session.rs:2091) must produce a *)
(* CONVERGENT, REACTIVATION-SAFE latch from these disagreeing streams:       *)
(*                                                                         *)
(*  1. CONVERGE-DOWN (audit F4 / completeness-critic #2). A dropped slot's   *)
(*     freeze `last_frame` converges DOWN to the global minimum across hosts *)
(*     -- at commit (`converged_drop_status`, :1351, folds the canonical     *)
(*     host's freeze by min over every other live host's staged freeze) and  *)
(*     on late arrival (`converge_latched_drop_status`, :1846, lowers the     *)
(*     latch for a host whose freeze undercuts it). The spectator replays     *)
(*     `inputs[last_frame]`, so `last_frame` must be a frame EVERY live host  *)
(*     confirmed; a freeze above the live global-min replays a value no       *)
(*     surviving host vouched for -> a silent spectator desync. The merge     *)
(*     NEVER raises a disconnected slot's freeze (`(true,true)` arm uses min, *)
(*     :2098), so a later canonical host with a higher view cannot undo it.   *)
(*                                                                         *)
(*  2. PROVENANCE-GATED REACTIVATION (audit critic-#1 / Session 31). A       *)
(*     genuine hot-join re-open (`disconnected -> connected`) must still be   *)
(*     followed (the `(true,false)` arm, :2101), so hot-join is tracked --     *)
(*     BUT only from a canonical host whose OWN gossip witnessed the latched  *)
(*     drop (`reactivation_provenance`, :1435: a per-host disconnect-witness  *)
(*     `host_drop_witness[h][p] >= latch.last_frame`). A stale lagging host   *)
(*     that never observed the drop and then BECOMES canonical (failover)     *)
(*     must NOT resurrect a permanently-dropped slot's label -- otherwise the  *)
(*     spectator silently replays live values for frames the mesh froze.      *)
(*                                                                         *)
(* PROPERTIES VERIFIED                                                      *)
(*   - Safety FreezeNeverRaised: while a slot stays latched-disconnected its *)
(*     freeze frame is non-increasing (the min/converge arms only lower it). *)
(*   - Safety LatchAtOrBelowLiveMin: a latched-disconnected slot's freeze is *)
(*     <= the global-min freeze across LIVE hosts that staged a drop -- the    *)
(*     desync-preventer (F4 / critic-#2).                                    *)
(*   - Safety NoFalseResurrection: a slot that was latched disconnected stays *)
(*     disconnected until a GENUINE reactivation occurs in the mesh -- a stale *)
(*     lagging canonical host that becomes canonical via failover but never    *)
(*     witnessed the drop can never re-open it (critic-#1 / Session 31). This  *)
(*     covers the STALE-LAGGING-CANONICAL (failover) class; the within-cycle    *)
(*     reordered-staging transient resurrection is out of model (see SCOPE).   *)
(*   - Safety GateAcceptsBoundaryWitness: the availability dual -- a genuine    *)
(*     current-drop witness at EXACTLY the converged freeze IS classified       *)
(*     Witnessed (the load-bearing `>=`, not `>`, in reactivation_provenance),  *)
(*     so a real hot-join re-open is not wrongly frozen out.                    *)
(*   - Liveness DropEventuallyLatched: a real drop is eventually reflected in *)
(*     the latch (or the player genuinely rejoins) -- the spectator does not   *)
(*     ignore a drop the hosts agree on.                                     *)
(*                                                                         *)
(* SCOPE / FAITHFUL SIMPLIFICATIONS (matching the production residual notes) *)
(*   - ONE droppable player. The production merge loops players independently *)
(*     (`for player_index in 0..num_players`, :1596/:1619) with no cross-player*)
(*     coupling, so one droppable slot captures the per-slot property in full.*)
(*   - ONE drop cycle, with an optional GENUINE rejoin (Connected -> Dropped   *)
(*     -> Rejoined), under IN-ORDER STAGING (see next bullet). The             *)
(*     merge_connection_status rustdoc (:2026-2090) documents three            *)
(*     degradations: (i) a FAIL-CLOSED window (safe -- keeps the frozen label); *)
(*     (ii) a CROSS-CYCLE fail-open that needs a drop-AFTER-a-rejoin (a second  *)
(*     cycle whose converged freeze a first cycle's stale witness numerically  *)
(*     covers, :2035-2055); and (iii) a WITHIN-CYCLE transient fail-open       *)
(*     (:2056-2067) that needs NO rejoin -- a genuine current-drop witness's    *)
(*     own REORDERED pre-drop connected snapshot, first-writer-staged at a      *)
(*     later frame, transiently resurrects the slot until its next drop-bearing *)
(*     packet re-adopts. This model excludes BOTH fail-open residuals: the      *)
(*     in-order-staging assumption rules out (iii)'s reordered staging, and the *)
(*     single drop cycle rules out (ii)'s second cycle. So NoFalseResurrection  *)
(*     here is the STALE-LAGGING-CANONICAL (failover) guarantee, NOT a universal *)
(*     no-resurrection claim -- (iii) is a single-cycle production fail-open this *)
(*     model does not see. Both fail-open residuals share one root cause        *)
(*     (connect-status reports carry no cycle/epoch identity) and one fix (a    *)
(*     host->spectator reactivation/epoch WIRE signal, tracked future work in   *)
(*     N-PLAYER-DESYNC-AUDIT.md); a reordering-aware multi-cycle spec that       *)
(*     reproduces them is the natural follow-on once that signal lands. The      *)
(*     witness `consume`/`re-arm` machinery (:1470/:1495) exists only to bound  *)
(*     cross-cycle provenance, so it is modeled faithfully but is NOT           *)
(*     load-bearing at this single-cycle bound and is NOT claimed to be pinned  *)
(*     (confirmed: removing either leaves every property GREEN).               *)
(*   - In-order per-host STAGING (a host's staged status goes Connected, then  *)
(*     Dropped, then Rejoined -- never out of order). This is an assumption     *)
(*     STRONGER than production's guarantee: production guarantees per-host     *)
(*     stream order (:2079), but first-writer-wins staging (:1778) can still    *)
(*     surface a reordered pre-drop snapshot at a later frame -- the residual    *)
(*     (iii) above. The assumption is what makes the provenance gate sound, so  *)
(*     it is the deliberate scope boundary, not an oversight.                   *)
(*   - The transient disconnect-PENDING window is abstracted to atomic removal: *)
(*     production keeps a host in `disconnecting_hosts` for one poll (skipped by *)
(*     canonical selection / both convergence folds, yet still staging+         *)
(*     witnessing); here a host is either fully `live` or gone. Sound for the   *)
(*     safety claims (a consume on the next follow clears any witness so armed). *)
(*   - The frame buffer / commit ORDERING is abstracted (proven in            *)
(*     SpectatorSession.tla): staging is "latest received status per host",   *)
(*     and a commit folds the canonical host's staged status into the latch.  *)
(*   - Input-VALUE divergence detection (`detect_snapshot_disagreement`,      *)
(*     :1543, and SpectatorDivergence) is a separate, complementary mechanism *)
(*     (it compares inputs, not connect-status); out of scope here, as        *)
(*     checksum/desync detection is out of scope in SpectatorSession.tla.     *)
(***************************************************************************)
EXTENDS Naturals, FiniteSets, TLC

CONSTANTS HOSTS,
    \* Ordered set of redundant hosts (e.g. {1,2,3});
    \* canonical = the lowest-numbered live host.
         MAX_FRAME,    \* Maximum freeze frame value for model checking.
         NULL_FRAME
\* Sentinel for "no witness" / initial (-1 in impl).
ASSUME HOSTS \subseteq Nat /\ HOSTS # {}
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME NULL_FRAME \notin 0 .. MAX_FRAME

(***************************************************************************)
(* Frame values. Drop freeze frames are real frames 1..MAX_FRAME (a host    *)
(* freezes the dropped slot at the last frame it received the peer's input);  *)
(* a connected/rejoined slot's last_frame is the baseline 0 (the value is     *)
(* irrelevant to every property, which all guard on `disconnected`).         *)
(***************************************************************************)
Frame == { NULL_FRAME } \union ( 0 .. MAX_FRAME )
FreezeFrame == 1 .. MAX_FRAME

\* a real drop freeze frame
CONNECTED_FRAME == 0
\* baseline last_frame for a connected slot
(***************************************************************************)
(* A host's (or the latch's) per-player connection status, mirroring the     *)
(* production `ConnectionStatus { disconnected, last_frame }`.               *)
(***************************************************************************)
Status == [disconnected:BOOLEAN, lastFrame:Frame ]
Connected == [ disconnected |-> FALSE, lastFrame |-> CONNECTED_FRAME ]

(***************************************************************************)
(* Lifecycle phases of the one droppable player. truePhase is the mesh       *)
(* ground truth; hostPhase[h] is host h's own (possibly lagging) view.       *)
(* Both advance only forward (in-order): Connected -> Dropped -> Rejoined.     *)
(***************************************************************************)
Phases == { "Connected", "Dropped", "Rejoined" }
PhaseRank(ph) == CASE ph = "Connected" -> 0
                 [] ph = "Dropped" -> 1
                 [] ph = "Rejoined" -> 2

(***************************************************************************)
(* Variables                                                               *)
(***************************************************************************)
VARIABLES
  truePhase,    \* The mesh's actual lifecycle phase for the dropped player.
  hostPhase,    \* [HOSTS -> Phases]: each host's own view (hostPhase[h] <= truePhase).
  hostFreeze,
    \* [HOSTS -> FreezeFrame]: host h's drop freeze frame (asymmetric,
    \* fixed at Init -- models per-host high-water received frame).
  live,
    \* SUBSET HOSTS: hosts not yet removed by failover (non-empty).
    \* --- spectator state (mirrors the production fields) ---
  latch,    \* Status: host_connect_status[player] -- the merged latched view.
  witness,    \* [HOSTS -> Frame]: host_drop_witness[h][player] (NULL = None).
  staged,
    \* [HOSTS -> Status]: latest received (staged) status per host.
    \* --- ghost (history) variables, for stating properties only ---
  latchEverDisc
\* TRUE once the latch has ever been disconnected.
vars ==
  << truePhase,
     hostPhase,
     hostFreeze,
     live,
     latch,
     witness,
     staged,
     latchEverDisc
  >>

(***************************************************************************)
(* The status host h currently gossips, derived from its phase view. A       *)
(* dropped host freezes at its own (asymmetric) freeze frame; a connected or  *)
(* rejoined host reports connected at the baseline frame.                    *)
(***************************************************************************)
HostReport(h) ==
  CASE hostPhase[h] = "Dropped" ->
  [ disconnected |-> TRUE, lastFrame |-> hostFreeze[h] ]
  [] OTHER-> Connected

(***************************************************************************)
(* Min over a NON-EMPTY set of frame numbers. CHOOSE ranges over frame        *)
(* VALUES only (never over HOSTS), so it is symmetry-safe -- though this spec   *)
(* uses no SYMMETRY anyway (host index is privileged: canonical = min live).  *)
(***************************************************************************)
Min(S) == CHOOSE x \in S: \A y \in S: x <= y

\* Binary min/max on frame numbers (witness/freeze bookkeeping).
Min2(x, y) == IF x <= y THEN x ELSE y
Max2(x, y) == IF x >= y THEN x ELSE y

(***************************************************************************)
(* Canonical host = the lowest-numbered LIVE host (production:                *)
(* `(0..hosts.len()).find(|i| !disconnect_pending(i))`, :1521). Failover       *)
(* removes a host from `live`, shifting the canonical to the next survivor.   *)
(***************************************************************************)
Canonical == Min(live)

(***************************************************************************)
(* Type Invariant                                                          *)
(***************************************************************************)
TypeInvariant ==
  /\ truePhase \in Phases
  /\ hostPhase \in [HOSTS -> Phases]
  /\ hostFreeze \in [HOSTS -> FreezeFrame]
  /\ live \subseteq HOSTS /\ live # {}
  /\ latch \in Status
  /\ witness \in [HOSTS -> Frame]
  /\ staged \in [HOSTS -> Status]
  /\ latchEverDisc \in BOOLEAN

(***************************************************************************)
(* Initial State: player connected everywhere, nothing staged/witnessed,     *)
(* all hosts live. hostFreeze is an adversarial fixed-at-Init asymmetric      *)
(* choice (each host's high-water received frame under packet loss).         *)
(***************************************************************************)
Init ==
  /\ truePhase = "Connected"
  /\ hostPhase = [h \in HOSTS |-> "Connected"]
  /\ hostFreeze \in [HOSTS -> FreezeFrame]
  /\ live = HOSTS
  /\ latch = Connected
  /\ witness = [h \in HOSTS |-> NULL_FRAME]
  /\ staged = [h \in HOSTS |-> Connected]
  /\ latchEverDisc = FALSE


(***************************************************************************)
(* ENVIRONMENT actions                                                      *)
(***************************************************************************)
(* The mesh ground truth advances one phase: the drop actually happens, then  *)
(* (optionally) the player genuinely rejoins. Monotone forward.              *)
AdvanceTruePhase ==
  /\ truePhase # "Rejoined"
  /\ truePhase' = IF truePhase = "Connected" THEN "Dropped" ELSE "Rejoined"
  /\ UNCHANGED << hostPhase,
        hostFreeze,
        live,
        latch,
        witness,
        staged,
        latchEverDisc
     >>

(* A live host advances its OWN view by one phase, never past the ground       *)
(* truth (a host cannot report a drop/rejoin that has not happened) and        *)
(* in-order (Connected -> Dropped -> Rejoined). This is the host observing the   *)
(* drop / the reactivation in its own stream; the spectator learns of it       *)
(* only via a subsequent ReceiveReport.                                       *)
AdvanceHostPhase(h) ==
  /\ h \in live
  /\ PhaseRank(hostPhase[h]) < PhaseRank(truePhase)
  /\ hostPhase' =
       [hostPhase EXCEPT
       ![h] =
       IF hostPhase[h] = "Connected" THEN "Dropped" ELSE "Rejoined"]
  /\ UNCHANGED << truePhase,
        hostFreeze,
        live,
        latch,
        witness,
        staged,
        latchEverDisc
     >>

(* Failover: a host times out and is removed. The spectator always keeps at    *)
(* least one host. Production removes the entry from `hosts` /                 *)
(* `host_snapshots` / `host_drop_witness` (index-parallel); here a removed     *)
(* host simply leaves `live`, so it is never canonical and never folded.       *)
Failover(h) ==
  /\ h \in live
  /\ live # { h }
  \* keep >= 1 host
  /\ live' = live \ { h }
  /\ UNCHANGED << truePhase,
        hostPhase,
        hostFreeze,
        latch,
        witness,
        staged,
        latchEverDisc
     >>


(***************************************************************************)
(* SPECTATOR actions                                                        *)
(***************************************************************************)
(* ReceiveReport(h): the spectator receives host h's current gossip (staging,  *)
(* `handle_host_input`, :1712). Faithful ordering:                            *)
(*   1. witness_host_drop_reports (:1751) BEFORE staging -- if the report is    *)
(*      disconnected, raise this host's witness to the reported freeze (max).  *)
(*   2. stage the report (host_snapshots; here `staged[h]`).                   *)
(*   3. converge_latched_drop_status (:1827) -- if the report's freeze         *)
(*      undercuts an already-disconnected latch, lower the latch (never raise).*)
ReceiveReport(h) ==
  /\ h \in live
  /\ LET report == HostReport(h)
     IN /\ witness' =
             [witness EXCEPT
             ![h] =
             IF report.disconnected
             THEN IF witness[h] = NULL_FRAME
               THEN report.lastFrame
               ELSE Max2(witness[h], report.lastFrame)
             ELSE witness[h]]
        /\ staged' = [staged EXCEPT ![h] = report]
        /\ latch' =
             IF report.disconnected /\ latch.disconnected /\
                 report.lastFrame < latch.lastFrame
             THEN [latch EXCEPT !.lastFrame = report.lastFrame]
             ELSE latch
  /\ UNCHANGED << truePhase, hostPhase, hostFreeze, live, latchEverDisc >>

(* CommitCanonical: the spectator commits the canonical host's STAGED status    *)
(* into the latch (`commit_canonical_snapshot`, :1591). For the player:        *)
(*   - converged_drop_status (:1648): if the canonical's staged status is       *)
(*     disconnected, fold its freeze DOWN by min over every OTHER live host's   *)
(*     staged disconnected freeze.                                            *)
(*   - reactivation_provenance (:1649): Witnessed iff the latch is disconnected *)
(*     AND this host's witness >= the latch freeze.                           *)
(*   - merge_connection_status (:1673), four arms (:2096):                     *)
(*       (T,T) -> last_frame := min   (converge down; NoTransition)            *)
(*       (T,F) -> if Witnessed: follow (latch := incoming; FollowedReactivation)*)
(*               else keep frozen     (NoTransition)                          *)
(*       (F,T) -> adopt the drop      (AdoptedDrop)                           *)
(*       (F,F) -> last_frame := max   (NoTransition)                          *)
(*   - witness bookkeeping (:1676): FollowedReactivation consumes every host's  *)
(*     witness for the player (:1683); AdoptedDrop re-arms the committing       *)
(*     host's witness at the adopted freeze (:1693).                          *)
ConvergedFreeze(c) ==
  LET others == {o \in live \ { c }: staged[o].disconnected}
  IN IF others = {}
      THEN staged[c].lastFrame
      ELSE Min({ staged[c].lastFrame } \union
            {staged[o].lastFrame:
              o \in others
            })

(***************************************************************************)
(* The provenance gate (`reactivation_provenance`, :1435): host c may have   *)
(* its `disconnected -> connected` report FOLLOWED iff the latch is disconnected*)
(* and c's own gossip witnessed the drop at a freeze frame >= the latched      *)
(* freeze. The `>=` (not `>`) is load-bearing for AVAILABILITY: a host that    *)
(* witnessed the drop at EXACTLY the converged freeze must still be able to    *)
(* re-open the slot (rustdoc :1413-1414) -- pinned by GateAcceptsBoundaryWitness.*)
(***************************************************************************)
Witnessed(c) ==
  /\ latch.disconnected
  /\ witness[c] # NULL_FRAME
  /\ witness[c] >= latch.lastFrame

CommitCanonical ==
  /\ LET c == Canonical
         incoming ==
           IF staged[c].disconnected
           THEN [ disconnected |-> TRUE, lastFrame |-> ConvergedFreeze(c) ]
           ELSE staged[c]
         witnessed == Witnessed(c)
     IN \/ \* (T,T): both disconnected -> converge down (min). NoTransition.
           /\ latch.disconnected /\ incoming.disconnected
           /\ latch' =
                [latch EXCEPT
                !.lastFrame =
                Min2(latch.lastFrame, incoming.lastFrame)]
           /\ UNCHANGED witness
        \/ \* (T,F): latch disconnected, incoming connected.
           /\ latch.disconnected /\ ~incoming.disconnected
           /\ \/ \* Witnessed -> FOLLOW the genuine reactivation; consume witnesses.
                 /\ witnessed
                 /\ latch' = incoming
                 /\ witness' = [h \in HOSTS |-> NULL_FRAME]
              \/ \* Unwitnessed -> keep the frozen label (the critic-#1 gate).
                 /\ ~witnessed
                 /\ UNCHANGED << latch, witness >>
        \/ \* (F,T): adopt a fresh drop; re-arm the committing host's witness.
           /\ ~latch.disconnected /\ incoming.disconnected
           /\ latch' = incoming
           /\ witness' =
                [witness EXCEPT
                ![c] =
                IF witness[c] = NULL_FRAME \/ incoming.lastFrame > witness[c]
                THEN incoming.lastFrame
                ELSE witness[c]]
        \/ \* (F,F): both connected -> monotone-up bookkeeping (max). NoTransition.
           /\ ~latch.disconnected /\ ~incoming.disconnected
           /\ latch' =
                [latch EXCEPT
                !.lastFrame =
                Max2(latch.lastFrame, incoming.lastFrame)]
           /\ UNCHANGED witness
  /\ latchEverDisc' = ( latchEverDisc \/ latch'.disconnected )
  /\ UNCHANGED << truePhase, hostPhase, hostFreeze, live, staged >>

(***************************************************************************)
(* Next-state relation                                                      *)
(***************************************************************************)
Next ==
  \/ AdvanceTruePhase
  \/ \E h \in HOSTS: AdvanceHostPhase(h)
  \/ \E h \in HOSTS: Failover(h)
  \/ \E h \in HOSTS: ReceiveReport(h)
  \/ CommitCanonical

(***************************************************************************)
(* Fairness: the spectator keeps receiving and committing, and hosts keep     *)
(* observing the truth. Drives the liveness property (a drop is eventually     *)
(* reflected). No fairness on Failover (it is an adversarial removal).        *)
(***************************************************************************)
Fairness ==
  /\ \A h \in HOSTS: WF_vars(AdvanceHostPhase(h))
  /\ \A h \in HOSTS: WF_vars(ReceiveReport(h))
  /\ WF_vars(CommitCanonical)

Spec == Init /\ [][Next]_vars /\ Fairness


(***************************************************************************)
(* SAFETY PROPERTIES                                                        *)
(***************************************************************************)
(* The set of LIVE hosts that have staged a disconnected report, and the       *)
(* global-min freeze across them.                                            *)
LiveDroppedStaged == {h \in live: staged[h].disconnected}
\* Partial: defined only when LiveDroppedStaged # {} (else Min CHOOSE's over the
\* empty set). Its sole caller LatchAtOrBelowLiveMin guards on that antecedent,
\* and the empty case (failover removed every host that staged the drop) IS
\* reachable -- so the guard is load-bearing, not cosmetic.
LiveMinFreeze == Min({staged[h].lastFrame: h \in LiveDroppedStaged})

(***************************************************************************)
(* LatchAtOrBelowLiveMin (audit F4 / completeness-critic #2) -- THE             *)
(* desync-preventer. A latched-disconnected slot's freeze is never above the   *)
(* global-min freeze across live hosts that staged a drop. The spectator        *)
(* replays `inputs[latch.lastFrame]`; bounding it by the live min guarantees    *)
(* every surviving host confirmed that frame, so no survivor disagrees with     *)
(* the replayed value (no silent desync). Folds down via converged_drop_status  *)
(* at commit and converge_latched_drop_status on late arrival; never raised.    *)
(***************************************************************************)
LatchAtOrBelowLiveMin ==
  ( latch.disconnected /\ LiveDroppedStaged # {} ) =>
    latch.lastFrame <= LiveMinFreeze

(***************************************************************************)
(* NoFalseResurrection (audit critic-#1 / Session 31) -- the provenance gate.   *)
(* Once the latch has been disconnected, it stays disconnected until the        *)
(* player GENUINELY rejoins in the mesh (truePhase = "Rejoined"). A stale        *)
(* lagging host that becomes canonical via failover but never witnessed the      *)
(* drop cannot re-open the slot. (Because truePhase is monotone, "not Rejoined   *)
(* yet" == "no genuine reactivation has happened", so a latch that is connected  *)
(* after having been disconnected, with truePhase # Rejoined, is exactly a       *)
(* FALSE resurrection.)                                                        *)
(***************************************************************************)
NoFalseResurrection ==
  ( latchEverDisc /\ truePhase # "Rejoined" ) => latch.disconnected

(***************************************************************************)
(* GateAcceptsBoundaryWitness -- the AVAILABILITY half of the provenance gate  *)
(* (the dual of NoFalseResurrection's soundness half). A genuine current-drop  *)
(* witness whose own freeze view sits at EXACTLY the converged latch freeze     *)
(* must be classified Witnessed, so its later genuine reactivation is FOLLOWED  *)
(* rather than wrongly frozen out -- the load-bearing `>=` (not `>`) in          *)
(* `reactivation_provenance` (rustdoc :1413-1414). Pins the gate's lower         *)
(* boundary: tightening `>=` to `>` (an availability regression -- a host that    *)
(* saw the drop at the freeze could never re-open the slot) makes this RED. The  *)
(* boundary state is reachable (an adopt re-arms `witness[c] = latch.lastFrame`   *)
(* exactly, then c rejoins and reports connected), so this is non-vacuous.       *)
(***************************************************************************)
GateAcceptsBoundaryWitness ==
  LET c == Canonical IN ( /\ latch.disconnected
                          /\ ~staged[c].disconnected
                          /\ witness[c] # NULL_FRAME
                          /\ witness[c] = latch.lastFrame ) => Witnessed(c)

SafetyInvariant ==
  /\ TypeInvariant
  /\ LatchAtOrBelowLiveMin
  /\ NoFalseResurrection
  /\ GateAcceptsBoundaryWitness

(***************************************************************************)
(* FreezeNeverRaised: while the latch stays disconnected across a step, its     *)
(* freeze frame is non-increasing. The min (`(true,true)`) and converge arms     *)
(* only lower it; no arm raises a disconnected slot's freeze (the production      *)
(* monotone-down guarantee a later higher-view canonical host cannot violate).   *)
(* An ACTION property: the antecedent excludes the connected->disconnected         *)
(* adopt step (latch.disconnected was FALSE there), where last_frame is set      *)
(* afresh rather than raised.                                                    *)
(***************************************************************************)
FreezeNeverRaised ==
  [][( latch.disconnected /\ latch'.disconnected ) =>
    latch'.lastFrame <= latch.lastFrame]_vars


(***************************************************************************)
(* LIVENESS                                                                 *)
(***************************************************************************)
(* DropEventuallyLatched: once the player has actually dropped, the spectator    *)
(* eventually reflects it (latch disconnected) -- unless the player has by then    *)
(* genuinely rejoined. The spectator does not silently ignore a drop the hosts    *)
(* agree on. Under fairness: a live host advances to Dropped, the spectator        *)
(* receives + commits, and the (F,T) adopt arm latches it.                        *)
(***************************************************************************)
DropEventuallyLatched ==
  ( truePhase = "Dropped" ) ~> ( latch.disconnected \/ truePhase = "Rejoined" )

(***************************************************************************)
(* State constraint: nothing unbounded (all variables are already finite);   *)
(* present for parity with the other specs / future bound tuning.           *)
(***************************************************************************)
StateConstraint == TRUE

(***************************************************************************)
(* Theorems                                                                *)
(***************************************************************************)
THEOREM SafetySpec == Spec => []SafetyInvariant
THEOREM FreezeSpec == Spec => FreezeNeverRaised
THEOREM LiveSpec == Spec => DropEventuallyLatched

=============================================================================
