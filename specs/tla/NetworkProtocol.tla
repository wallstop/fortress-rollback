--------------------------- MODULE NetworkProtocol ---------------------------
(***************************************************************************)
(* TLA+ Specification for Fortress Rollback Network Protocol               *)
(*                                                                         *)
(* This module specifies the UDP protocol state machine used for peer-to-  *)
(* peer communication in Fortress Rollback. It models:                     *)
(*   - Protocol state transitions (Initializing -> Synchronizing -> etc.)  *)
(*   - Synchronization handshake                                           *)
(*   - Message passing between peers                                       *)
(*   - Timeout and disconnection handling                                  *)
(*                                                                         *)
(* Properties verified:                                                    *)
(*   - Safety: No invalid state transitions                                *)
(*   - Liveness: Eventually synchronized (under fair scheduling)           *)
(*   - No deadlocks                                                        *)
(***************************************************************************)

EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    NUM_SYNC_PACKETS,       \* Number of sync roundtrips required (5)
    PEERS                   \* Set of peer identifiers

ASSUME NUM_SYNC_PACKETS \in Nat /\ NUM_SYNC_PACKETS > 0
ASSUME PEERS # {}

(***************************************************************************)
(* Protocol States                                                         *)
(***************************************************************************)
ProtocolStates == {"Initializing", "Synchronizing", "Running", "Disconnected", "Shutdown"}

(***************************************************************************)
(* Message Types                                                           *)
(***************************************************************************)
MessageTypes == {"SyncRequest", "SyncReply", "Input", "InputAck", "KeepAlive"}

(***************************************************************************)
(* Variables                                                               *)
(***************************************************************************)
VARIABLES
    state,              \* state[p] = protocol state for peer p
    syncRemaining,      \* syncRemaining[p] = remaining sync packets for peer p
    syncRequests,       \* syncRequests[p] = set of pending sync request IDs
    network,            \* network = sequence of messages in transit
    disconnectTimer,    \* disconnectTimer[p] = timer for disconnect detection
    shutdownTimer       \* shutdownTimer[p] = timer for shutdown transition

vars == <<state, syncRemaining, syncRequests, network, disconnectTimer, shutdownTimer>>

(***************************************************************************)
(* Type Invariant                                                          *)
(***************************************************************************)
TypeInvariant ==
    /\ state \in [PEERS -> ProtocolStates]
    /\ syncRemaining \in [PEERS -> Nat]
    /\ syncRequests \in [PEERS -> SUBSET Nat]
    /\ network \in Seq([type: MessageTypes, from: PEERS, to: PEERS, data: Nat])
    /\ disconnectTimer \in [PEERS -> Nat]
    /\ shutdownTimer \in [PEERS -> Nat]

(***************************************************************************)
(* Initial State                                                           *)
(***************************************************************************)
Init ==
    /\ state = [p \in PEERS |-> "Initializing"]
    /\ syncRemaining = [p \in PEERS |-> NUM_SYNC_PACKETS]
    /\ syncRequests = [p \in PEERS |-> {}]
    /\ network = <<>>
    /\ disconnectTimer = [p \in PEERS |-> 0]
    /\ shutdownTimer = [p \in PEERS |-> 0]

(***************************************************************************)
(* Helper: Send a message                                                  *)
(***************************************************************************)
Send(msg) == network' = Append(network, msg)

(***************************************************************************)
(* Helper: Receive a message (non-deterministic selection)                 *)
(***************************************************************************)
Receive(p) ==
    /\ Len(network) > 0
    /\ \E i \in 1..Len(network):
        /\ network[i].to = p
        /\ network' = SubSeq(network, 1, i-1) \o SubSeq(network, i+1, Len(network))

(***************************************************************************)
(* Action: Start synchronization                                           *)
(* Transition: Initializing -> Synchronizing                               *)
(***************************************************************************)
StartSync(p, other) ==
    /\ state[p] = "Initializing"
    /\ state' = [state EXCEPT ![p] = "Synchronizing"]
    /\ \E randomId \in 1..1000:
        /\ syncRequests' = [syncRequests EXCEPT ![p] = syncRequests[p] \union {randomId}]
        /\ Send([type |-> "SyncRequest", from |-> p, to |-> other, data |-> randomId])
    /\ UNCHANGED <<syncRemaining, disconnectTimer, shutdownTimer>>

(***************************************************************************)
(* Action: Handle SyncRequest - send SyncReply                             *)
(***************************************************************************)
HandleSyncRequest(p) ==
    /\ Len(network) > 0
    /\ \E i \in 1..Len(network):
        /\ network[i].to = p
        /\ network[i].type = "SyncRequest"
        /\ LET msg == network[i] IN
            /\ Send([type |-> "SyncReply", from |-> p, to |-> msg.from, data |-> msg.data])
            /\ network' = Append(
                SubSeq(network, 1, i-1) \o SubSeq(network, i+1, Len(network)),
                [type |-> "SyncReply", from |-> p, to |-> msg.from, data |-> msg.data])
    /\ UNCHANGED <<state, syncRemaining, syncRequests, disconnectTimer, shutdownTimer>>

(***************************************************************************)
(* Action: Handle SyncReply - decrement sync counter                       *)
(* Transition: Synchronizing -> Running (when counter reaches 0)           *)
(***************************************************************************)
HandleSyncReply(p) ==
    /\ state[p] = "Synchronizing"
    /\ Len(network) > 0
    /\ \E i \in 1..Len(network):
        /\ network[i].to = p
        /\ network[i].type = "SyncReply"
        /\ network[i].data \in syncRequests[p]
        /\ LET msg == network[i] IN
            /\ syncRequests' = [syncRequests EXCEPT ![p] = syncRequests[p] \ {msg.data}]
            /\ syncRemaining' = [syncRemaining EXCEPT ![p] = syncRemaining[p] - 1]
            /\ IF syncRemaining[p] - 1 = 0
               THEN state' = [state EXCEPT ![p] = "Running"]
               ELSE state' = state
            /\ network' = SubSeq(network, 1, i-1) \o SubSeq(network, i+1, Len(network))
    /\ UNCHANGED <<disconnectTimer, shutdownTimer>>

(***************************************************************************)
(* Action: Send another sync request (retry)                               *)
(***************************************************************************)
RetrySyncRequest(p, other) ==
    /\ state[p] = "Synchronizing"
    /\ syncRemaining[p] > 0
    /\ \E randomId \in 1..1000:
        /\ randomId \notin syncRequests[p]
        /\ syncRequests' = [syncRequests EXCEPT ![p] = syncRequests[p] \union {randomId}]
        /\ Send([type |-> "SyncRequest", from |-> p, to |-> other, data |-> randomId])
    /\ UNCHANGED <<state, syncRemaining, disconnectTimer, shutdownTimer>>

(***************************************************************************)
(* Action: Timeout in Running state -> Disconnected                        *)
(***************************************************************************)
DisconnectTimeout(p) ==
    /\ state[p] = "Running"
    /\ disconnectTimer[p] > 10  \* Simplified timeout threshold
    /\ state' = [state EXCEPT ![p] = "Disconnected"]
    /\ UNCHANGED <<syncRemaining, syncRequests, network, disconnectTimer, shutdownTimer>>

(***************************************************************************)
(* Action: Receive message resets disconnect timer                         *)
(***************************************************************************)
ReceiveKeepAlive(p) ==
    /\ state[p] = "Running"
    /\ Len(network) > 0
    /\ \E i \in 1..Len(network):
        /\ network[i].to = p
        /\ network' = SubSeq(network, 1, i-1) \o SubSeq(network, i+1, Len(network))
    /\ disconnectTimer' = [disconnectTimer EXCEPT ![p] = 0]
    /\ UNCHANGED <<state, syncRemaining, syncRequests, shutdownTimer>>

(***************************************************************************)
(* Action: Increment disconnect timer (time passes)                        *)
(***************************************************************************)
Tick(p) ==
    /\ state[p] \in {"Running", "Disconnected"}
    /\ IF state[p] = "Running"
       THEN disconnectTimer' = [disconnectTimer EXCEPT ![p] = disconnectTimer[p] + 1]
       ELSE disconnectTimer' = disconnectTimer
    /\ IF state[p] = "Disconnected"
       THEN shutdownTimer' = [shutdownTimer EXCEPT ![p] = shutdownTimer[p] + 1]
       ELSE shutdownTimer' = shutdownTimer
    /\ UNCHANGED <<state, syncRemaining, syncRequests, network>>

(***************************************************************************)
(* Action: Shutdown timer expires -> Shutdown                              *)
(***************************************************************************)
ShutdownTimeout(p) ==
    /\ state[p] = "Disconnected"
    /\ shutdownTimer[p] > 5  \* Simplified timeout threshold
    /\ state' = [state EXCEPT ![p] = "Shutdown"]
    /\ UNCHANGED <<syncRemaining, syncRequests, network, disconnectTimer, shutdownTimer>>

(***************************************************************************)
(* Action: Explicit shutdown from any state                                *)
(***************************************************************************)
ExplicitShutdown(p) ==
    /\ state[p] # "Shutdown"
    /\ state' = [state EXCEPT ![p] = "Shutdown"]
    /\ UNCHANGED <<syncRemaining, syncRequests, network, disconnectTimer, shutdownTimer>>

(***************************************************************************)
(* Next State Relation                                                     *)
(***************************************************************************)
Next ==
    \E p \in PEERS:
        \E other \in PEERS \ {p}:
            \/ StartSync(p, other)
            \/ HandleSyncRequest(p)
            \/ HandleSyncReply(p)
            \/ RetrySyncRequest(p, other)
            \/ DisconnectTimeout(p)
            \/ ReceiveKeepAlive(p)
            \/ Tick(p)
            \/ ShutdownTimeout(p)
            \/ ExplicitShutdown(p)

(***************************************************************************)
(* Fairness Conditions                                                     *)
(***************************************************************************)
Fairness ==
    /\ \A p \in PEERS: WF_vars(HandleSyncReply(p))
    /\ \A p \in PEERS: \A other \in PEERS \ {p}: WF_vars(HandleSyncRequest(p))

(***************************************************************************)
(* Specification                                                           *)
(***************************************************************************)
Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Properties                                                       *)
(***************************************************************************)

\* SAFE-1: Valid state transitions only
ValidStateTransitions ==
    [][\A p \in PEERS:
        \/ state[p] = state'[p]  \* No change
        \/ (state[p] = "Initializing" /\ state'[p] = "Synchronizing")
        \/ (state[p] = "Synchronizing" /\ state'[p] = "Running")
        \/ (state[p] = "Running" /\ state'[p] = "Disconnected")
        \/ (state[p] = "Disconnected" /\ state'[p] = "Shutdown")
        \/ state'[p] = "Shutdown"  \* Can always shutdown
    ]_vars

\* SAFE-2: Sync remaining never negative
SyncRemainingNonNegative ==
    \A p \in PEERS: syncRemaining[p] >= 0

\* SAFE-3: Only Running state can process game inputs
OnlyRunningProcessesInputs ==
    \A p \in PEERS:
        state[p] # "Running" =>
            ~\E i \in 1..Len(network):
                network[i].to = p /\ network[i].type = "Input"

(***************************************************************************)
(* Liveness Properties                                                     *)
(***************************************************************************)

\* LIVE-1: Eventually synchronized (if not shutdown)
EventuallySynchronized ==
    \A p \in PEERS:
        (state[p] = "Synchronizing") ~> (state[p] = "Running" \/ state[p] = "Shutdown")

\* LIVE-2: No deadlock - always some action possible or all shutdown
NoDeadlock ==
    (\A p \in PEERS: state[p] = "Shutdown") \/ ENABLED(Next)

(***************************************************************************)
(* Invariants to Check                                                     *)
(***************************************************************************)
Invariants ==
    /\ TypeInvariant
    /\ SyncRemainingNonNegative

=============================================================================
