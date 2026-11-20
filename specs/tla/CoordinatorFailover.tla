------------------------ MODULE CoordinatorFailover ------------------------
(***************************************************************************)
(* Coordinator-failover policy for coordinated graceful drop (#497).        *)
(*                                                                         *)
(* The production protocol keeps the deterministic lowest participant as   *)
(* the coordinator. When that participant is partitioned away from a       *)
(* reachable strict majority, no participant can admit the operation or    *)
(* choose a cut, so the majority currently fail-closes. This model         *)
(* abstracts the failover policy: after a bounded grace the lowest         *)
(* remaining participant (FALLBACK) acts as coordinator, admits the        *)
(* operation, collects inventory reports from a strict-majority quorum,     *)
(* and chooses a non-retracting cut. A still-reachable real coordinator     *)
(* may choose a competing cut in the safety configuration; the model       *)
(* checks that at most one cut can ever be committed.                      *)
(*                                                                         *)
(* The operational envelope is intentionally narrow: one operation, one    *)
(* fallback candidate, fixed receipts, and a single ready transcript per    *)
(* peer. Quorum intersection plus the stable `readyCut` transcript is the  *)
(* load-bearing safety argument; the Rust implementation additionally      *)
(* validates participant sets, generations, cut digests, backfill, and     *)
(* target acknowledgements.                                               *)
(*                                                                         *)
(* Safety (`CoordinatorFailover.cfg`, COORD_AVAILABLE = TRUE):            *)
(*   AtMostOneCommittedCut, CommittedMatchesReady, CutCoversReceipt,       *)
(*   SingleReadyTranscript.                                                *)
(*                                                                         *)
(* Liveness (`CoordinatorFailoverLive.cfg`, COORD_AVAILABLE = FALSE):     *)
(*   EventuallyFailoverCommits -- with the coordinator partitioned away,  *)
(*   the fallback quorum eventually commits a cut.                        *)
(***************************************************************************)

EXTENDS Naturals, FiniteSets, TLC

CONSTANTS
    PEERS,
    COORD,
    FALLBACK,
    QUORUM,
    MAX_RECEIPT,
    MAX_CUT,
    COORD_AVAILABLE,
    NO_CUT

ASSUME Cardinality(PEERS) = 4
ASSUME COORD \in PEERS
ASSUME FALLBACK \in PEERS
ASSUME COORD # FALLBACK
ASSUME QUORUM = 3
ASSUME MAX_RECEIPT \in Nat
ASSUME MAX_CUT \in Nat
ASSUME COORD_AVAILABLE \in BOOLEAN
ASSUME NO_CUT \notin 0..MAX_CUT

FallbackGroup == PEERS \ {COORD}
Cuts == 0..MAX_CUT
Receipts == 0..MAX_RECEIPT
Phases == {"Running", "Prepared", "Committed"}

VARIABLES
    phase,
    reported,
    receipt,
    grace,
    acting,
    cut,
    cutChosen,
    ready,
    readyCut,
    committed,
    commitCut,
    coordinatorCut,
    coordinatorCutChosen

vars == <<phase, reported, receipt, grace, acting, cut, cutChosen,
          ready, readyCut, committed, commitCut,
          coordinatorCut, coordinatorCutChosen>>

Init ==
    /\ phase = [p \in PEERS |-> "Running"]
    /\ reported = [p \in PEERS |-> FALSE]
    /\ receipt = [p \in PEERS |-> 0]
    /\ grace = FALSE
    /\ acting = FALSE
    /\ cut = NO_CUT
    /\ cutChosen = FALSE
    /\ ready = [p \in PEERS |-> FALSE]
    /\ readyCut = [p \in PEERS |-> NO_CUT]
    /\ committed = [p \in PEERS |-> FALSE]
    /\ commitCut = [p \in PEERS |-> NO_CUT]
    /\ coordinatorCut = NO_CUT
    /\ coordinatorCutChosen = FALSE

ElapseGrace ==
    /\ ~grace
    /\ grace' = TRUE
    /\ UNCHANGED <<phase, reported, receipt, acting, cut, cutChosen,
                   ready, readyCut, committed, commitCut,
                   coordinatorCut, coordinatorCutChosen>>

Takeover ==
    /\ grace
    /\ ~acting
    /\ acting' = TRUE
    /\ UNCHANGED <<phase, reported, receipt, grace, cut, cutChosen,
                   ready, readyCut, committed, commitCut,
                   coordinatorCut, coordinatorCutChosen>>

Prepare(p) ==
    /\ acting
    /\ p \in FallbackGroup
    /\ phase[p] = "Running"
    /\ phase' = [phase EXCEPT ![p] = "Prepared"]
    /\ UNCHANGED <<reported, receipt, grace, acting, cut, cutChosen,
                   ready, readyCut, committed, commitCut,
                   coordinatorCut, coordinatorCutChosen>>

Report(p, r) ==
    /\ phase[p] \in {"Running", "Prepared"}
    /\ ~reported[p]
    /\ r \in Receipts
    /\ reported' = [reported EXCEPT ![p] = TRUE]
    /\ receipt' = [receipt EXCEPT ![p] = r]
    /\ UNCHANGED <<phase, grace, acting, cut, cutChosen,
                   ready, readyCut, committed, commitCut,
                   coordinatorCut, coordinatorCutChosen>>

ChooseFallbackCut(c) ==
    /\ acting
    /\ ~cutChosen
    /\ c \in Cuts
    /\ \A p \in FallbackGroup: reported[p] /\ c >= receipt[p]
    /\ cut' = c
    /\ cutChosen' = TRUE
    /\ UNCHANGED <<phase, reported, receipt, grace, acting,
                   ready, readyCut, committed, commitCut,
                   coordinatorCut, coordinatorCutChosen>>

ChooseCoordinatorCut(c) ==
    /\ COORD_AVAILABLE
    /\ ~coordinatorCutChosen
    /\ c \in Cuts
    /\ coordinatorCut' = c
    /\ coordinatorCutChosen' = TRUE
    /\ UNCHANGED <<phase, reported, receipt, grace, acting, cut, cutChosen,
                   ready, readyCut, committed, commitCut>>

ReadyFallback(p) ==
    /\ cutChosen
    /\ p \in FallbackGroup
    /\ reported[p]
    /\ ~ready[p]
    /\ cut >= receipt[p]
    /\ ready' = [ready EXCEPT ![p] = TRUE]
    /\ readyCut' = [readyCut EXCEPT ![p] = cut]
    /\ UNCHANGED <<phase, reported, receipt, grace, acting, cut, cutChosen,
                   committed, commitCut, coordinatorCut,
                   coordinatorCutChosen>>

ReadyCoordinator(p) ==
    /\ coordinatorCutChosen
    /\ reported[p]
    /\ ~ready[p]
    /\ coordinatorCut >= receipt[p]
    /\ ready' = [ready EXCEPT ![p] = TRUE]
    /\ readyCut' = [readyCut EXCEPT ![p] = coordinatorCut]
    /\ UNCHANGED <<phase, reported, receipt, grace, acting, cut, cutChosen,
                   committed, commitCut, coordinatorCut,
                   coordinatorCutChosen>>

ReadyCount(c) ==
    Cardinality({p \in PEERS: ready[p] /\ readyCut[p] = c})

Commit(p) ==
    /\ ready[p]
    /\ readyCut[p] # NO_CUT
    /\ ReadyCount(readyCut[p]) >= QUORUM
    /\ ~committed[p]
    /\ committed' = [committed EXCEPT ![p] = TRUE]
    /\ commitCut' = [commitCut EXCEPT ![p] = readyCut[p]]
    /\ UNCHANGED <<phase, reported, receipt, grace, acting, cut, cutChosen,
                   ready, readyCut, coordinatorCut, coordinatorCutChosen>>

Next ==
    \/ ElapseGrace
    \/ Takeover
    \/ \E p \in FallbackGroup: Prepare(p)
    \/ \E p \in PEERS, r \in Receipts: Report(p, r)
    \/ \E c \in Cuts: ChooseFallbackCut(c)
    \/ \E c \in Cuts: ChooseCoordinatorCut(c)
    \/ \E p \in FallbackGroup: ReadyFallback(p)
    \/ \E p \in PEERS: ReadyCoordinator(p)
    \/ \E p \in PEERS: Commit(p)

Spec == Init /\ [][Next]_vars
LiveSpec == Init /\ [][Next]_vars /\ WF_vars(Next)

TypeInvariant ==
    /\ phase \in [PEERS -> Phases]
    /\ reported \in [PEERS -> BOOLEAN]
    /\ receipt \in [PEERS -> Receipts]
    /\ grace \in BOOLEAN
    /\ acting \in BOOLEAN
    /\ cut \in Cuts \union {NO_CUT}
    /\ cutChosen \in BOOLEAN
    /\ ready \in [PEERS -> BOOLEAN]
    /\ readyCut \in [PEERS -> Cuts \union {NO_CUT}]
    /\ committed \in [PEERS -> BOOLEAN]
    /\ commitCut \in [PEERS -> Cuts \union {NO_CUT}]
    /\ coordinatorCut \in Cuts \union {NO_CUT}
    /\ coordinatorCutChosen \in BOOLEAN

SingleReadyTranscript ==
    \A p \in PEERS:
        ready[p] => readyCut[p] # NO_CUT

CutCoversReceipt ==
    \A p \in PEERS:
        ready[p] => readyCut[p] >= receipt[p]

CommittedMatchesReady ==
    \A p \in PEERS:
        committed[p] => /\ ready[p] /\ commitCut[p] = readyCut[p]

AtMostOneCommittedCut ==
    \A p, q \in PEERS:
        committed[p] /\ committed[q] => commitCut[p] = commitCut[q]

EventuallyFailoverCommits ==
    <> (\E p \in FallbackGroup: committed[p])

SafetyInvariant ==
    /\ TypeInvariant
    /\ SingleReadyTranscript
    /\ CutCoversReceipt
    /\ CommittedMatchesReady
    /\ AtMostOneCommittedCut

=============================================================================
