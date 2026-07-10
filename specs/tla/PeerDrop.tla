------------------------------- MODULE PeerDrop -------------------------------
(***************************************************************************)
(* TLA+ model for peer-drop policy semantics.                              *)
(*                                                                         *)
(* This intentionally models the policy contract at a small state-machine  *)
(* level instead of the full UDP protocol:                                 *)
(*   - Halt fails closed by moving the session to Synchronizing            *)
(*   - ContinueWithout freezes dropped players and keeps survivors running *)
(*   - PeerDropped events are only emitted by ContinueWithout              *)
(*   - Dropped players are excluded from survivor progress                 *)
(*   - Rollback starts no later than every dropped player's lastFrame + 1  *)
(***************************************************************************)

EXTENDS Naturals, FiniteSets, Sequences, TLC

CONSTANTS
    PLAYERS,
    MAX_FRAME,
    NULL_FRAME

ASSUME PLAYERS # {}
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME NULL_FRAME \notin 0..MAX_FRAME

SessionStates == {"Running", "Synchronizing"}
Policies == {"Halt", "ContinueWithout"}
EventTypes == {"PeerDropped", "Disconnected"}
Frame == {NULL_FRAME} \union (0..MAX_FRAME)
Event == [type: EventTypes, player: PLAYERS]

VARIABLES
    state,
    policy,
    currentFrame,
    rawConfirmedFrame,
    confirmedFrame,
    haltCeiling,
    connected,
    dropped,
    frozen,
    lastFrame,
    rollbackFrame,
    events

vars == <<state, policy, currentFrame, rawConfirmedFrame, confirmedFrame, haltCeiling,
          connected, dropped, frozen,
          lastFrame, rollbackFrame, events>>

TypeInvariant ==
    /\ state \in SessionStates
    /\ policy \in Policies
    /\ currentFrame \in 0..MAX_FRAME
    /\ rawConfirmedFrame \in 0..currentFrame
    /\ confirmedFrame \in 0..MAX_FRAME
    /\ haltCeiling \in Frame
    /\ connected \in [PLAYERS -> BOOLEAN]
    /\ dropped \in [PLAYERS -> BOOLEAN]
    /\ frozen \in [PLAYERS -> BOOLEAN]
    /\ lastFrame \in [PLAYERS -> 0..MAX_FRAME]
    /\ rollbackFrame \in Frame
    /\ events \in Seq(Event)

Survivors == {p \in PLAYERS: connected[p] /\ ~dropped[p]}
DroppedPlayers == {p \in PLAYERS: dropped[p]}
Cutoff(p) == IF lastFrame[p] < MAX_FRAME THEN lastFrame[p] + 1 ELSE MAX_FRAME

AppendDropEvents(seq, p) ==
    Append(
        Append(seq, [type |-> "PeerDropped", player |-> p]),
        [type |-> "Disconnected", player |-> p]
    )

Init ==
    /\ state = "Running"
    /\ policy \in Policies
    /\ currentFrame = 0
    /\ rawConfirmedFrame = 0
    /\ confirmedFrame = 0
    /\ haltCeiling = NULL_FRAME
    /\ connected = [p \in PLAYERS |-> TRUE]
    /\ dropped = [p \in PLAYERS |-> FALSE]
    /\ frozen = [p \in PLAYERS |-> FALSE]
    /\ lastFrame = [p \in PLAYERS |-> 0]
    /\ rollbackFrame = NULL_FRAME
    /\ events = <<>>

(***************************************************************************)
(* Normal survivor advancement. Dropped players do not participate in the  *)
(* lastFrame minimum/progress computation.                                 *)
(***************************************************************************)
Advance ==
    /\ state = "Running"
    /\ currentFrame < MAX_FRAME
    /\ Survivors # {}
    /\ currentFrame' = currentFrame + 1
    /\ rawConfirmedFrame' \in rawConfirmedFrame..(currentFrame + 1)
    /\ confirmedFrame' = rawConfirmedFrame'
    /\ lastFrame' = [p \in PLAYERS |->
        IF p \in Survivors THEN currentFrame + 1 ELSE lastFrame[p]]
    /\ UNCHANGED <<state, policy, haltCeiling, connected, dropped, frozen,
                   rollbackFrame, events>>

(***************************************************************************)
(* A direct or propagated remote drop.                                     *)
(***************************************************************************)
DropPeer(p) ==
    /\ \/ state = "Running"
       \/ (policy = "Halt" /\ state = "Synchronizing")
    /\ p \in PLAYERS
    /\ connected[p]
    /\ ~dropped[p]
    /\ connected' = [connected EXCEPT ![p] = FALSE]
    /\ dropped' = [dropped EXCEPT ![p] = TRUE]
    /\ IF policy = "Halt"
       THEN
           /\ state' = "Synchronizing"
           /\ haltCeiling' =
               IF haltCeiling = NULL_FRAME \/ confirmedFrame < haltCeiling
               THEN confirmedFrame
               ELSE haltCeiling
           /\ frozen' = frozen
           /\ rollbackFrame' = rollbackFrame
           /\ events' = Append(events, [type |-> "Disconnected", player |-> p])
       ELSE
           /\ state' = "Running"
           /\ haltCeiling' = haltCeiling
           /\ frozen' = [frozen EXCEPT ![p] = TRUE]
           /\ rollbackFrame' =
               IF rollbackFrame = NULL_FRAME \/ Cutoff(p) < rollbackFrame
               THEN Cutoff(p)
               ELSE rollbackFrame
           /\ events' = AppendDropEvents(events, p)
    /\ UNCHANGED <<policy, currentFrame, rawConfirmedFrame, confirmedFrame, lastFrame>>

(***************************************************************************)
(* The defect-producing fold can rise after Halt because dropped slots are *)
(* no longer members. The public value must remain capped even while this   *)
(* underlying raw bound moves into the speculative window.                 *)
(***************************************************************************)
PostHaltFoldRecompute ==
    /\ state = "Synchronizing"
    /\ policy = "Halt"
    /\ haltCeiling # NULL_FRAME
    /\ rawConfirmedFrame < currentFrame
    /\ rawConfirmedFrame' \in (rawConfirmedFrame + 1)..currentFrame
    /\ confirmedFrame' =
        IF rawConfirmedFrame' <= haltCeiling
        THEN rawConfirmedFrame'
        ELSE haltCeiling
    /\ UNCHANGED <<state, policy, currentFrame, haltCeiling, connected,
                   dropped, frozen, lastFrame, rollbackFrame, events>>

(***************************************************************************)
(* Late knowledge of an already dropped player is ignored.                 *)
(***************************************************************************)
LateDropKnowledge(p) ==
    /\ p \in PLAYERS
    /\ dropped[p]
    /\ UNCHANGED vars

Next ==
    \/ Advance
    \/ \E p \in PLAYERS: DropPeer(p)
    \/ PostHaltFoldRecompute
    \/ \E p \in PLAYERS: LateDropKnowledge(p)

Spec == Init /\ [][Next]_vars

(***************************************************************************)
(* Safety properties                                                       *)
(***************************************************************************)

HaltFailsClosed ==
    policy = "Halt" /\ DroppedPlayers # {} => state = "Synchronizing"

HaltConfirmationFrozen ==
    policy = "Halt" /\ haltCeiling # NULL_FRAME =>
        confirmedFrame <= haltCeiling

ReportedConfirmationCapped ==
    confirmedFrame =
        IF haltCeiling = NULL_FRAME
        THEN rawConfirmedFrame
        ELSE IF rawConfirmedFrame <= haltCeiling
             THEN rawConfirmedFrame
             ELSE haltCeiling

ContinueWithoutFreezesDropped ==
    policy = "ContinueWithout" =>
        \A p \in PLAYERS: dropped[p] => frozen[p]

PeerDroppedOnlyForContinueWithout ==
    \A i \in 1..Len(events):
        events[i].type = "PeerDropped" => policy = "ContinueWithout"

DroppedPlayersExcludedFromSurvivors ==
    \A p \in PLAYERS: dropped[p] => p \notin Survivors

RollbackStartsAtEarliestDrop ==
    policy = "ContinueWithout" /\ rollbackFrame # NULL_FRAME =>
        \A p \in PLAYERS: dropped[p] => rollbackFrame <= Cutoff(p)

SafetyInvariant ==
    /\ TypeInvariant
    /\ HaltFailsClosed
    /\ HaltConfirmationFrozen
    /\ ReportedConfirmationCapped
    /\ ContinueWithoutFreezesDropped
    /\ PeerDroppedOnlyForContinueWithout
    /\ DroppedPlayersExcludedFromSurvivors
    /\ RollbackStartsAtEarliestDrop

StateConstraint ==
    /\ currentFrame <= MAX_FRAME
    /\ Len(events) <= 2 * Cardinality(PLAYERS)

THEOREM Spec => []SafetyInvariant

=============================================================================
