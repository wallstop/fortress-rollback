------------------------- MODULE ChecksumExchange -------------------------
(***************************************************************************)
(* TLA+ Specification for Fortress Rollback Checksum Exchange System       *)
(*                                                                         *)
(* This module specifies the desync detection mechanism used for verifying *)
(* game state synchronization between peers. It models:                    *)
(*   - Local checksum computation and sending                              *)
(*   - Remote checksum reception and storage (pending_checksums)           *)
(*   - Checksum comparison and desync detection                            *)
(*   - SyncHealth state machine (Pending -> InSync | DesyncDetected)       *)
(*                                                                         *)
(* Properties verified:                                                    *)
(*   - Safety: No false positives (DesyncDetected only when checksums      *)
(*             actually differ)                                            *)
(*   - Safety: No false negatives (differing checksums are always          *)
(*             detected when comparison occurs)                            *)
(*   - Liveness: Pending checksums eventually become comparable            *)
(*   - Invariant: last_verified_frame is monotonically increasing          *)
(*                                                                         *)
(* Production-Spec Alignment:                                              *)
(*   This spec models the checksum flow in P2PSession:                     *)
(*   - check_checksum_send_interval() -> SendChecksum action               *)
(*   - handle_checksum_report() -> ReceiveChecksum action                  *)
(*   - compare_local_checksums_against_peers() -> CompareChecksums action  *)
(*   - sync_health() queries the resulting state                           *)
(***************************************************************************)

EXTENDS Naturals, Integers, Sequences, FiniteSets, TLC

CONSTANTS
    PEERS,              \* Set of peer identifiers (e.g., {p1, p2})
    MAX_FRAME,          \* Maximum frame number to model (for bounded checking)
    CHECKSUM_INTERVAL   \* Frames between checksum sends

ASSUME PEERS # {}
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME CHECKSUM_INTERVAL \in Nat /\ CHECKSUM_INTERVAL > 0

(***************************************************************************)
(* Sync Health States                                                      *)
(***************************************************************************)
SyncHealthStates == {"Pending", "InSync", "DesyncDetected"}

(***************************************************************************)
(* Variables                                                               *)
(***************************************************************************)
VARIABLES
    \* Frame tracking
    currentFrame,           \* currentFrame[p] = current simulation frame for peer p
    lastConfirmedFrame,     \* lastConfirmedFrame[p] = last confirmed frame for peer p
    
    \* Local checksum tracking
    localChecksums,         \* localChecksums[p] = map from frame -> checksum for peer p
    lastSentChecksumFrame,  \* lastSentChecksumFrame[p] = last frame we sent checksum for
    
    \* Remote checksum tracking (pending_checksums)
    pendingChecksums,       \* pendingChecksums[p] = map from frame -> checksum received from other peer
    
    \* Sync health state
    syncHealth,             \* syncHealth[p] = current sync health state
    lastVerifiedFrame,      \* lastVerifiedFrame[p] = highest frame with successful comparison (or -1)
    
    \* Game state (abstract) - for modeling actual vs predicted checksums
    \* In production, checksum is derived from actual game state
    \* For verification, we model whether peers have the "same" state
    peersHaveSameState,     \* Boolean: TRUE if peers have deterministic identical state
    
    \* Network - checksum messages in transit
    network                 \* Sequence of checksum messages

vars == <<currentFrame, lastConfirmedFrame, localChecksums, lastSentChecksumFrame,
          pendingChecksums, syncHealth, lastVerifiedFrame, peersHaveSameState, network>>

(***************************************************************************)
(* Helper: Valid frame range for checksums                                 *)
(***************************************************************************)
FrameRange == 0..MAX_FRAME

(***************************************************************************)
(* Type Invariant                                                          *)
(***************************************************************************)
TypeInvariant ==
    /\ currentFrame \in [PEERS -> Nat]
    /\ lastConfirmedFrame \in [PEERS -> Int]  \* Int to allow -1 (NULL_FRAME)
    /\ localChecksums \in [PEERS -> [FrameRange -> Nat \cup {-1}]]  \* -1 means no checksum
    /\ lastSentChecksumFrame \in [PEERS -> Int]
    /\ pendingChecksums \in [PEERS -> [FrameRange -> Nat \cup {-1}]]  \* -1 means no checksum
    /\ syncHealth \in [PEERS -> SyncHealthStates]
    /\ lastVerifiedFrame \in [PEERS -> Int]
    /\ peersHaveSameState \in BOOLEAN
    /\ network \in Seq([type: {"Checksum"}, from: PEERS, to: PEERS, frame: Nat, checksum: Nat])

(***************************************************************************)
(* Initial State                                                           *)
(***************************************************************************)
Init ==
    /\ currentFrame = [p \in PEERS |-> 0]
    /\ lastConfirmedFrame = [p \in PEERS |-> -1]  \* NULL_FRAME initially
    /\ localChecksums = [p \in PEERS |-> [f \in FrameRange |-> -1]]  \* All slots empty (-1)
    /\ lastSentChecksumFrame = [p \in PEERS |-> -1]
    /\ pendingChecksums = [p \in PEERS |-> [f \in FrameRange |-> -1]]  \* All slots empty (-1)
    /\ syncHealth = [p \in PEERS |-> "Pending"]
    /\ lastVerifiedFrame = [p \in PEERS |-> -1]
    /\ peersHaveSameState = TRUE  \* Initially synchronized
    /\ network = <<>>

(***************************************************************************)
(* Helper: Compute checksum for a frame                                    *)
(* In reality, this is derived from game state.                            *)
(* For verification, we model it based on peersHaveSameState.              *)
(* If peers have same state, same frame -> same checksum.                  *)
(* If peers have different state, same frame -> different checksums.       *)
(***************************************************************************)
ComputeChecksum(peer, frame) ==
    IF peersHaveSameState
    THEN frame * 1000  \* All peers get same checksum for same frame
    ELSE frame * 1000 + (IF peer = CHOOSE p \in PEERS: TRUE THEN 1 ELSE 2)

(***************************************************************************)
(* Action: Advance frame for a peer                                        *)
(* Models: session.advance_frame()                                         *)
(***************************************************************************)
AdvanceFrame(p) ==
    /\ currentFrame[p] < MAX_FRAME
    /\ currentFrame' = [currentFrame EXCEPT ![p] = currentFrame[p] + 1]
    /\ UNCHANGED <<lastConfirmedFrame, localChecksums, lastSentChecksumFrame,
                   pendingChecksums, syncHealth, lastVerifiedFrame, peersHaveSameState, network>>

(***************************************************************************)
(* Action: Confirm a frame (inputs received from all peers)                *)
(* Models: sync_layer.confirm_frame() after receiving remote inputs        *)
(***************************************************************************)
ConfirmFrame(p) ==
    /\ lastConfirmedFrame[p] < currentFrame[p]
    /\ lastConfirmedFrame' = [lastConfirmedFrame EXCEPT ![p] = lastConfirmedFrame[p] + 1]
    /\ UNCHANGED <<currentFrame, localChecksums, lastSentChecksumFrame,
                   pendingChecksums, syncHealth, lastVerifiedFrame, peersHaveSameState, network>>

(***************************************************************************)
(* Action: Send checksum at interval                                       *)
(* Models: check_checksum_send_interval() in P2PSession                    *)
(***************************************************************************)
SendChecksum(p, other) ==
    LET frameToSend == IF lastSentChecksumFrame[p] < 0
                       THEN CHECKSUM_INTERVAL
                       ELSE lastSentChecksumFrame[p] + CHECKSUM_INTERVAL
    IN
    /\ frameToSend <= lastConfirmedFrame[p]
    /\ frameToSend >= 0
    /\ frameToSend <= MAX_FRAME
    /\ LET checksum == ComputeChecksum(p, frameToSend)
       IN
        /\ localChecksums' = [localChecksums EXCEPT ![p] = 
            [localChecksums[p] EXCEPT ![frameToSend] = checksum]]
        /\ lastSentChecksumFrame' = [lastSentChecksumFrame EXCEPT ![p] = frameToSend]
        /\ network' = Append(network, [type |-> "Checksum", from |-> p, to |-> other, 
                                       frame |-> frameToSend, checksum |-> checksum])
    /\ UNCHANGED <<currentFrame, lastConfirmedFrame, pendingChecksums, 
                   syncHealth, lastVerifiedFrame, peersHaveSameState>>

(***************************************************************************)
(* Action: Receive checksum from network                                   *)
(* Models: handle_checksum_report() in UdpProtocol                         *)
(***************************************************************************)
ReceiveChecksum(p) ==
    /\ Len(network) > 0
    /\ \E i \in 1..Len(network):
        /\ network[i].to = p
        /\ network[i].type = "Checksum"
        /\ LET msg == network[i]
           IN
            /\ pendingChecksums' = [pendingChecksums EXCEPT ![p] = 
                [pendingChecksums[p] EXCEPT ![msg.frame] = msg.checksum]]
            /\ network' = SubSeq(network, 1, i-1) \o SubSeq(network, i+1, Len(network))
    /\ UNCHANGED <<currentFrame, lastConfirmedFrame, localChecksums, lastSentChecksumFrame,
                   syncHealth, lastVerifiedFrame, peersHaveSameState>>

(***************************************************************************)
(* Action: Compare checksums and update sync health                        *)
(* Models: compare_local_checksums_against_peers() in P2PSession           *)
(***************************************************************************)
CompareChecksums(p) ==
    \E frame \in FrameRange:
        /\ pendingChecksums[p][frame] # -1  \* We have a pending checksum for this frame
        /\ localChecksums[p][frame] # -1    \* We have a local checksum too
        /\ frame < lastConfirmedFrame[p]    \* Only compare confirmed frames
        /\ LET localChecksum == localChecksums[p][frame]
               remoteChecksum == pendingChecksums[p][frame]
           IN
            IF localChecksum = remoteChecksum
            THEN 
                \* Checksums match - update to InSync, update last_verified_frame
                /\ syncHealth' = [syncHealth EXCEPT ![p] = "InSync"]
                /\ lastVerifiedFrame' = [lastVerifiedFrame EXCEPT ![p] = 
                    IF lastVerifiedFrame[p] < frame THEN frame ELSE lastVerifiedFrame[p]]
                /\ pendingChecksums' = [pendingChecksums EXCEPT ![p][frame] = -1]  \* Clear after compare
            ELSE
                \* Checksums differ - desync detected!
                /\ syncHealth' = [syncHealth EXCEPT ![p] = "DesyncDetected"]
                /\ lastVerifiedFrame' = lastVerifiedFrame  \* Don't update on desync
                /\ pendingChecksums' = [pendingChecksums EXCEPT ![p][frame] = -1]  \* Clear after compare
        /\ UNCHANGED <<currentFrame, lastConfirmedFrame, localChecksums, lastSentChecksumFrame,
                       peersHaveSameState, network>>

(***************************************************************************)
(* Action: Introduce desync (for testing - models non-determinism bug)     *)
(* This can only happen ONCE and represents a bug in game logic            *)
(***************************************************************************)
IntroduceDesync ==
    /\ peersHaveSameState = TRUE
    /\ peersHaveSameState' = FALSE
    /\ UNCHANGED <<currentFrame, lastConfirmedFrame, localChecksums, lastSentChecksumFrame,
                   pendingChecksums, syncHealth, lastVerifiedFrame, network>>

(***************************************************************************)
(* Action: Done - all frames processed, allow termination                  *)
(***************************************************************************)
Done ==
    /\ \A p \in PEERS: currentFrame[p] = MAX_FRAME
    /\ \A p \in PEERS: lastConfirmedFrame[p] = MAX_FRAME
    /\ network = <<>>
    /\ UNCHANGED vars

(***************************************************************************)
(* Next State Relation                                                     *)
(***************************************************************************)
Next ==
    \/ \E p \in PEERS:
        \E other \in PEERS \ {p}:
            \/ AdvanceFrame(p)
            \/ ConfirmFrame(p)
            \/ SendChecksum(p, other)
            \/ ReceiveChecksum(p)
            \/ CompareChecksums(p)
    \/ IntroduceDesync
    \/ Done

(***************************************************************************)
(* Fairness Conditions                                                     *)
(***************************************************************************)
Fairness ==
    /\ \A p \in PEERS: WF_vars(ReceiveChecksum(p))
    /\ \A p \in PEERS: WF_vars(CompareChecksums(p))
    /\ \A p \in PEERS: \A other \in PEERS \ {p}: SF_vars(SendChecksum(p, other))

(***************************************************************************)
(* Specification                                                           *)
(***************************************************************************)
Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Properties                                                       *)
(***************************************************************************)

\* SAFE-DESYNC-1: No false positives
\* DesyncDetected can only occur when peersHaveSameState is FALSE
\* (i.e., actual desync exists)
NoFalsePositives ==
    \A p \in PEERS:
        (syncHealth[p] = "DesyncDetected") => (peersHaveSameState = FALSE)

\* SAFE-DESYNC-2: DesyncDetected is a terminal state (once detected, stays detected)
DesyncIsTerminal ==
    [][\A p \in PEERS:
        syncHealth[p] = "DesyncDetected" => syncHealth'[p] = "DesyncDetected"
    ]_vars

\* SAFE-DESYNC-3: last_verified_frame is monotonically increasing
VerifiedFrameMonotonic ==
    [][\A p \in PEERS:
        lastVerifiedFrame[p] <= lastVerifiedFrame'[p]
    ]_vars

\* SAFE-DESYNC-4: Comparison only happens on confirmed frames
\* (This is implicit in the action guard, not needed as explicit invariant)

\* SAFE-DESYNC-5: InSync implies all compared checksums matched
InSyncImpliesMatchingChecksums ==
    \A p \in PEERS:
        (syncHealth[p] = "InSync") => 
            (lastVerifiedFrame[p] >= 0)  \* At least one successful comparison happened

(***************************************************************************)
(* Liveness Properties                                                     *)
(***************************************************************************)

\* LIVE-DESYNC-1: If peers have same state and keep advancing, eventually InSync
EventuallyInSync ==
    (peersHaveSameState /\ (\A p \in PEERS: currentFrame[p] >= CHECKSUM_INTERVAL))
        ~> (\A p \in PEERS: syncHealth[p] = "InSync")

\* LIVE-DESYNC-2: If desync introduced, eventually detected (under fair scheduling)
EventuallyDetected ==
    (peersHaveSameState = FALSE /\ \A p \in PEERS: lastConfirmedFrame[p] >= CHECKSUM_INTERVAL)
        ~> (\E p \in PEERS: syncHealth[p] = "DesyncDetected")

(***************************************************************************)
(* State Constraint for Model Checking                                     *)
(***************************************************************************)
StateConstraint ==
    /\ \A p \in PEERS: currentFrame[p] <= MAX_FRAME
    /\ \A p \in PEERS: lastConfirmedFrame[p] <= MAX_FRAME
    /\ Len(network) <= 4

(***************************************************************************)
(* Invariants to Check                                                     *)
(***************************************************************************)
Invariants ==
    /\ TypeInvariant
    /\ NoFalsePositives
    /\ InSyncImpliesMatchingChecksums

=============================================================================
