------------------------ MODULE CoordinatorSplitView ------------------------
(***************************************************************************)
(* Split-view participant-set safety for coordinated graceful drop (#504).  *)
(*                                                                         *)
(* Two requesters in one split view can each mint the SAME operation id for *)
(* the same target while deriving DIFFERENT participant vectors from their  *)
(* locally reachable endpoints. Each group is then a strict majority of its *)
(* own four-member view (`{coordinator} + group`), so both can commit a     *)
(* different cut for one target and fork the target's confirmed history.    *)
(*                                                                         *)
(* The production fix derives the participant set from the agreed           *)
(* membership epoch (every configured Local/Remote slot that has not been   *)
(* certified dropped), independent of local endpoint reachability, and      *)
(* binds its digest into the operation identity. This model abstracts the   *)
(* two requester groups and checks the resulting quorum rule:               *)
(*                                                                         *)
(* - `DERIVE_LOCAL_VIEW = FALSE` (the shipped policy): both groups derive   *)
(*   the shared epoch, so at most one group can hold a strict majority.     *)
(*   `AtMostOneCommittedCut` is non-vacuous: GROUP_B commits and GROUP_F    *)
(*   cannot.                                                                *)
(* - `DERIVE_LOCAL_VIEW = TRUE` (`CoordinatorSplitViewLocal.cfg`, manual    *)
(*   demo): each group derives `its group + coordinator`, so both can       *)
(*   choose and commit a different cut. TLC reports the fork.               *)
(*                                                                         *)
(* Scope: one target, one operation, group-internal all-ready abstraction.  *)
(* Operation identity binding, participant-digest validation, and the       *)
(* explicit-removal CAP trade-off (a minority now fails closed instead of   *)
(* certifying) are Rust-level obligations pinned by the deterministic       *)
(* split-view simulation row and the participant-derivation unit guards.    *)
(***************************************************************************)

EXTENDS Naturals, FiniteSets, TLC

CONSTANTS
    GROUP_B,
    GROUP_F,
    COORD,
    MAX_CUT,
    DERIVE_LOCAL_VIEW,
    NO_CUT

Groups == {"B", "F"}

GroupSet(g) == IF g = "B" THEN GROUP_B ELSE GROUP_F

Epoch == GROUP_B \union GROUP_F \union COORD

Quorum(S) == (Cardinality(S) \div 2) + 1

ASSUME GROUP_B # {}
ASSUME GROUP_F # {}
ASSUME GROUP_B \cap GROUP_F = {}
ASSUME GROUP_B \cap COORD = {}
ASSUME GROUP_F \cap COORD = {}
ASSUME MAX_CUT \in Nat
ASSUME DERIVE_LOCAL_VIEW \in BOOLEAN
ASSUME NO_CUT \notin 0..MAX_CUT

VARIABLES
    view,
    cutChosen,
    cut,
    committed,
    commitCut

vars == <<view, cutChosen, cut, committed, commitCut>>

Init ==
    /\ view = [g \in Groups |-> {}]
    /\ cutChosen = [g \in Groups |-> FALSE]
    /\ cut = [g \in Groups |-> NO_CUT]
    /\ committed = [g \in Groups |-> FALSE]
    /\ commitCut = [g \in Groups |-> NO_CUT]

\* Both groups derive their participant set at operation start.
Derive(g) ==
    /\ view[g] = {}
    /\ view' = [view EXCEPT ![g] =
            IF DERIVE_LOCAL_VIEW
            THEN GroupSet(g) \union COORD
            ELSE Epoch]
    /\ UNCHANGED <<cutChosen, cut, committed, commitCut>>

\* A group may pick a cut only with a strict majority of the view it derived.
ChooseCut(g, c) ==
    /\ view[g] # {}
    /\ ~cutChosen[g]
    /\ Cardinality(GroupSet(g)) >= Quorum(view[g])
    /\ c \in 0..MAX_CUT
    /\ cutChosen' = [cutChosen EXCEPT ![g] = TRUE]
    /\ cut' = [cut EXCEPT ![g] = c]
    /\ UNCHANGED <<view, committed, commitCut>>

Commit(g) ==
    /\ cutChosen[g]
    /\ ~committed[g]
    /\ committed' = [committed EXCEPT ![g] = TRUE]
    /\ commitCut' = [commitCut EXCEPT ![g] = cut[g]]
    /\ UNCHANGED <<view, cutChosen, cut>>

\* A group can still make progress: derive its view, choose a cut if its
\* group is a strict majority of the view, or commit a chosen cut.
CanAct(g) ==
    \/ view[g] = {}
    \/ (~cutChosen[g] /\ Cardinality(GroupSet(g)) >= Quorum(view[g]))
    \/ (cutChosen[g] /\ ~committed[g])

\* Quiescence: every group has committed or can no longer act. The stuttering
\* step is enabled only here, so weak fairness of Next cannot mask a reachable
\* commit behind an infinite pre-commit stutter.
Terminal ==
    /\ \A g \in Groups: committed[g] \/ ~CanAct(g)
    /\ UNCHANGED vars

Next ==
    \/ \E g \in Groups: Derive(g)
    \/ \E g \in Groups, c \in 0..MAX_CUT: ChooseCut(g, c)
    \/ \E g \in Groups: Commit(g)
    \/ Terminal

Spec == Init /\ [][Next]_vars /\ WF_vars(Next)

TypeInvariant ==
    /\ view \in [Groups -> SUBSET Epoch]
    /\ cutChosen \in [Groups -> BOOLEAN]
    /\ cut \in [Groups -> 0..MAX_CUT \union {NO_CUT}]
    /\ committed \in [Groups -> BOOLEAN]
    /\ commitCut \in [Groups -> 0..MAX_CUT \union {NO_CUT}]

CommittedMatchesCut ==
    \A g \in Groups: committed[g] => /\ cutChosen[g] /\ commitCut[g] = cut[g]

\* One target and one operation: two committed cuts must coincide.
AtMostOneCommittedCut ==
    \A g, h \in Groups:
        committed[g] /\ committed[h] => commitCut[g] = commitCut[h]

EventuallyOneGroupCommits ==
    <> (\E g \in Groups: committed[g])

SafetyInvariant ==
    /\ TypeInvariant
    /\ CommittedMatchesCut
    /\ AtMostOneCommittedCut

=============================================================================
