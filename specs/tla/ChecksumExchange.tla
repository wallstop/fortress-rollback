------------------------- MODULE ChecksumExchange -------------------------
(***************************************************************************)
(* TLA+ Specification for Fortress Rollback Checksum Exchange System       *)
(*                                                                         *)
(* This module specifies the desync detection mechanism used for verifying *)
(* game state synchronization between peers. It models:                    *)
(*   - Local checksum computation and broadcast to every remote            *)
(*   - Remote checksum reception and per-endpoint storage                  *)
(*     (UdpProtocol::pending_checksums)                                    *)
(*   - Per-(local,remote)-pair checksum comparison and desync detection    *)
(*   - Per-pair SyncHealth verdict (Pending -> InSync | DesyncDetected)    *)
(*                                                                         *)
(* PAIR KEYING (re-modeled after Session 27 found the old [PEERS -> ...]   *)
(* keying unfaithful at N=3): all verdict state is keyed by ORDERED pairs  *)
(* of distinct peers. syncHealth[p][q] is p's verdict about remote q,      *)
(* mirroring the implementation, which keeps `pending_checksums` and       *)
(* `last_verified_frame` on each per-remote UdpProtocol endpoint (the F12  *)
(* fix, src/sessions/p2p_session.rs / src/network/protocol/mod.rs). With   *)
(* the old per-local keying, a match against one remote clobbered a desync *)
(* verdict from another remote at N=3, violating DesyncIsTerminal -- a     *)
(* spec-modeling bug the implementation never had. Self-pairs do not       *)
(* exist: every pair function has inner domain PEERS \ {p}.                *)
(*                                                                         *)
(* Properties verified:                                                    *)
(*   - Safety: No false positives, pair-precise (DesyncDetected for (p,q)  *)
(*             only when one of p,q actually diverged)                     *)
(*   - Safety: Desync verdicts are terminal per pair (no cross-pair        *)
(*             clobbering -- the Session 27 regression guard)              *)
(*   - Safety: last_verified_frame is per-pair monotonically increasing    *)
(*   - Safety (F12-shaped): InSync for (p,q) requires a checksum match in  *)
(*             pair (p,q) itself; aggregate synchronization requires EVERY *)
(*             pair individually verified (no cross-peer leakage)          *)
(*                                                                         *)
(* Production-Spec Alignment (src/sessions/p2p_session.rs unless noted):   *)
(*   - check_checksum_send_interval() -> SendChecksum action (broadcasts   *)
(*     one frame's checksum to ALL remotes, single per-session             *)
(*     last_sent_checksum_frame / local_checksum_history)                  *)
(*   - UdpProtocol::on_checksum_report() -> ReceiveChecksum action         *)
(*     (stores into the RECEIVING endpoint's pending_checksums)            *)
(*   - compare_local_checksums_against_peers() -> CompareChecksums action  *)
(*     (per-remote-endpoint comparison; match bumps THAT endpoint's        *)
(*     last_verified_frame; mismatch emits a DesyncDetected event)         *)
(*   - sync_health(handle) -> syncHealth[p][q] (per-pair verdict)          *)
(*   - is_synchronized() -> IsSynchronized(p) (all remotes individually    *)
(*     InSync)                                                             *)
(*   - last_verified_frame() -> AggregateLastVerified(p) (min over         *)
(*     remotes, undefined while any remote is unverified)                  *)
(*                                                                         *)
(* Deliberate abstractions (also unmodeled in the previous N=2 version):   *)
(*   - syncHealth[p][q] LATCHES DesyncDetected. In the implementation the  *)
(*     DesyncDetected FortressEvent emission is monotone -- an emitted     *)
(*     event is never retracted, though a client that lets the bounded     *)
(*     event queue overflow can EVICT an unread DesyncDetected event       *)
(*     (p2p_session.rs, max_event_queue_size pop_front), an abstraction    *)
(*     this spec does not model -- and the documented contract is to       *)
(*     terminate on it;                                                    *)
(*     the derived sync_health() query itself can report InSync again      *)
(*     after the mismatched pending entry is consumed if an older          *)
(*     pre-divergence checksum verified the pair. The spec models the      *)
(*     latched, event-level verdict.                                       *)
(*   - No disconnects / reserved hot-join endpoints: every remote counts   *)
(*     toward the aggregates (the impl filters by remote_is_connected).    *)
(*   - No pending_checksums max_history eviction (protocol/mod.rs          *)
(*     on_checksum_report retain) and no local_checksum_history trimming.  *)
(*   - No last_saved_frame gate or disconnect-rollback (M1) send deferral. *)
(*   - Divergence is a one-shot event hitting a single peer; the other     *)
(*     peers continue to agree with each other.                            *)
(***************************************************************************)

EXTENDS Naturals, Integers, FiniteSets, TLC

CONSTANTS
    PEERS,              \* Set of peer identifiers (e.g., {p1, p2, p3})
    MAX_FRAME,          \* Maximum frame number to model (for bounded checking)
    CHECKSUM_INTERVAL   \* Frames between checksum sends

ASSUME PEERS # {} /\ Cardinality(PEERS) >= 2
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME CHECKSUM_INTERVAL \in Nat /\ CHECKSUM_INTERVAL > 0

(***************************************************************************)
(* Symmetry: the spec is invariant under permutations of PEERS -- every    *)
(* action and every checked property quantifies uniformly over peers, and  *)
(* ComputeChecksum depends on a peer only through `peer = divergedPeer`.   *)
(* TLC's symmetry reduction is unsound in combination with CHOOSE over     *)
(* symmetric model values; the ONLY CHOOSE in this module is in            *)
(* AggregateLastVerified, which ranges over a set of Int frame numbers     *)
(* (never over PEERS or peer-keyed structures) and is uniquely determined  *)
(* (the minimum), so symmetry is sound. Liveness checking (which TLC       *)
(* forbids under symmetry) is disabled in the .cfg. Used via SYMMETRY in   *)
(* ChecksumExchange.cfg; cuts the N=3 state space by up to |PEERS|! = 6x.  *)
(***************************************************************************)
Symm == Permutations(PEERS)

(***************************************************************************)
(* Sync Health States                                                      *)
(***************************************************************************)
SyncHealthStates == {"Pending", "InSync", "DesyncDetected"}

\* Sentinel for "no peer has diverged". A TLC string compares unequal to
\* every model value in PEERS, so PEERS \cup {NoDivergence} is well-formed.
NoDivergence == "none"

(***************************************************************************)
(* Variables                                                               *)
(***************************************************************************)
VARIABLES
    \* Frame tracking (per local peer)
    currentFrame,           \* currentFrame[p] = current simulation frame for peer p
    lastConfirmedFrame,     \* lastConfirmedFrame[p] = last confirmed frame for peer p

    \* Local checksum tracking (per local peer; the impl keeps ONE
    \* local_checksum_history / last_sent_checksum_frame per session and
    \* broadcasts the same checksum to every remote)
    localChecksums,         \* localChecksums[p] = frame -> checksum (local_checksum_history)
    lastSentChecksumFrame,  \* lastSentChecksumFrame[p] = last frame p sent a checksum for

    \* Per-(local,remote) PAIR state (per-remote UdpProtocol endpoint state)
    pendingChecksums,       \* pendingChecksums[p][q] = frame -> checksum p received FROM q
    syncHealth,             \* syncHealth[p][q] = p's latched sync verdict about q
    lastVerifiedFrame,      \* lastVerifiedFrame[p][q] = highest frame where p's local
                            \*   checksum matched a checksum from q (or -1)

    \* Game state (abstract). Models a one-shot non-determinism bug hitting
    \* a single peer: that peer's post-divergence checksums differ from
    \* everyone else's, while the remaining peers still agree.
    divergedPeer,           \* Peer whose state diverged, or NoDivergence

    \* Network - checksum messages in transit (a SET: UDP delivery is
    \* unordered, and messages are unique records, so no ordering state)
    network

vars == <<currentFrame, lastConfirmedFrame, localChecksums, lastSentChecksumFrame,
          pendingChecksums, syncHealth, lastVerifiedFrame, divergedPeer, network>>

(***************************************************************************)
(* Helpers                                                                 *)
(***************************************************************************)
FrameRange == 0..MAX_FRAME

\* The remotes of local peer p (ordered-pair partners; self-pairs never exist)
Remotes(p) == PEERS \ {p}

(***************************************************************************)
(* Type Invariant                                                          *)
(***************************************************************************)
TypeInvariant ==
    /\ currentFrame \in [PEERS -> Nat]
    /\ lastConfirmedFrame \in [PEERS -> Int]  \* Int to allow -1 (NULL_FRAME)
    /\ localChecksums \in [PEERS -> [FrameRange -> Nat \cup {-1}]]  \* -1 means no checksum
    /\ lastSentChecksumFrame \in [PEERS -> Int]
    /\ DOMAIN pendingChecksums = PEERS
    /\ \A p \in PEERS:
        /\ DOMAIN pendingChecksums[p] = Remotes(p)
        /\ \A q \in Remotes(p): pendingChecksums[p][q] \in [FrameRange -> Nat \cup {-1}]
    /\ DOMAIN syncHealth = PEERS
    /\ \A p \in PEERS:
        /\ DOMAIN syncHealth[p] = Remotes(p)
        /\ \A q \in Remotes(p): syncHealth[p][q] \in SyncHealthStates
    /\ DOMAIN lastVerifiedFrame = PEERS
    /\ \A p \in PEERS:
        /\ DOMAIN lastVerifiedFrame[p] = Remotes(p)
        /\ \A q \in Remotes(p): lastVerifiedFrame[p][q] \in Int
    /\ divergedPeer \in PEERS \cup {NoDivergence}
    /\ \A m \in network:
        m \in [type: {"Checksum"}, from: PEERS, to: PEERS, frame: FrameRange, checksum: Nat]

(***************************************************************************)
(* Initial State                                                           *)
(***************************************************************************)
Init ==
    /\ currentFrame = [p \in PEERS |-> 0]
    /\ lastConfirmedFrame = [p \in PEERS |-> -1]  \* NULL_FRAME initially
    /\ localChecksums = [p \in PEERS |-> [f \in FrameRange |-> -1]]
    /\ lastSentChecksumFrame = [p \in PEERS |-> -1]
    /\ pendingChecksums = [p \in PEERS |-> [q \in Remotes(p) |-> [f \in FrameRange |-> -1]]]
    /\ syncHealth = [p \in PEERS |-> [q \in Remotes(p) |-> "Pending"]]
    /\ lastVerifiedFrame = [p \in PEERS |-> [q \in Remotes(p) |-> -1]]
    /\ divergedPeer = NoDivergence
    /\ network = {}

(***************************************************************************)
(* Helper: Compute checksum for a frame                                    *)
(* In reality, this is derived from game state. For verification we model  *)
(* it from divergence: while no peer diverged, every peer computes the     *)
(* same checksum for the same frame; after divergence, only the diverged   *)
(* peer's checksums differ (the healthy peers still agree).                *)
(***************************************************************************)
ComputeChecksum(peer, frame) ==
    frame * 1000 + (IF peer = divergedPeer THEN 1 ELSE 0)

(***************************************************************************)
(* Action: Advance frame for a peer                                        *)
(* Models: session.advance_frame()                                         *)
(***************************************************************************)
AdvanceFrame(p) ==
    /\ currentFrame[p] < MAX_FRAME
    /\ currentFrame' = [currentFrame EXCEPT ![p] = currentFrame[p] + 1]
    /\ UNCHANGED <<lastConfirmedFrame, localChecksums, lastSentChecksumFrame,
                   pendingChecksums, syncHealth, lastVerifiedFrame, divergedPeer, network>>

(***************************************************************************)
(* Action: Confirm a frame (inputs received from all peers)                *)
(* Models: sync_layer.confirm_frame() after receiving remote inputs        *)
(***************************************************************************)
ConfirmFrame(p) ==
    /\ lastConfirmedFrame[p] < currentFrame[p]
    /\ lastConfirmedFrame' = [lastConfirmedFrame EXCEPT ![p] = lastConfirmedFrame[p] + 1]
    /\ UNCHANGED <<currentFrame, localChecksums, lastSentChecksumFrame,
                   pendingChecksums, syncHealth, lastVerifiedFrame, divergedPeer, network>>

(***************************************************************************)
(* Action: Send checksum at interval                                       *)
(* Models: check_checksum_send_interval() in P2PSession. The impl computes *)
(* ONE checksum per interval frame and broadcasts it to EVERY remote       *)
(* endpoint (`for remote in remotes { remote.send_checksum_report(..) }`), *)
(* recording it once in local_checksum_history -- so this is a single      *)
(* broadcast action, not a per-target send.                                *)
(***************************************************************************)
SendChecksum(p) ==
    LET frameToSend == IF lastSentChecksumFrame[p] < 0
                       THEN CHECKSUM_INTERVAL
                       ELSE lastSentChecksumFrame[p] + CHECKSUM_INTERVAL
    IN
    /\ frameToSend <= lastConfirmedFrame[p]
    /\ frameToSend >= 0
    /\ frameToSend <= MAX_FRAME
    /\ LET checksum == ComputeChecksum(p, frameToSend)
       IN
        /\ localChecksums' = [localChecksums EXCEPT ![p][frameToSend] = checksum]
        /\ lastSentChecksumFrame' = [lastSentChecksumFrame EXCEPT ![p] = frameToSend]
        /\ network' = network \cup
            {[type |-> "Checksum", from |-> p, to |-> q,
              frame |-> frameToSend, checksum |-> checksum] : q \in Remotes(p)}
    /\ UNCHANGED <<currentFrame, lastConfirmedFrame, pendingChecksums,
                   syncHealth, lastVerifiedFrame, divergedPeer>>

(***************************************************************************)
(* Action: Receive checksum from network                                   *)
(* Models: UdpProtocol::on_checksum_report() -- the report is stored in    *)
(* the pending_checksums of the endpoint it arrived on, i.e. keyed by the  *)
(* SENDER, never merged across remotes.                                    *)
(***************************************************************************)
ReceiveChecksum(p) ==
    \E m \in network:
        /\ m.to = p
        /\ pendingChecksums' =
            [pendingChecksums EXCEPT ![p][m.from][m.frame] = m.checksum]
        /\ network' = network \ {m}
        /\ UNCHANGED <<currentFrame, lastConfirmedFrame, localChecksums,
                       lastSentChecksumFrame, syncHealth, lastVerifiedFrame, divergedPeer>>

(***************************************************************************)
(* Action: Compare checksums for ONE (local, remote) pair                  *)
(* Models: compare_local_checksums_against_peers() in P2PSession. The impl *)
(* iterates every remote endpoint and every confirmed pending frame in one *)
(* call; the spec performs one (pair, frame) comparison per step, which is *)
(* the same set of individual comparisons under interleaving.              *)
(*   - Match: bump THAT endpoint's last_verified_frame (per-pair; the F12  *)
(*     fix -- verification of q never leaks into r's verdict).             *)
(*   - Mismatch: a DesyncDetected event is emitted for this pair; the      *)
(*     latched verdict becomes (and stays) DesyncDetected.                 *)
(*   - Either way the pending entry is consumed (checked_frames removal).  *)
(* A later match cannot clear a latched DesyncDetected: event emission is  *)
(* monotone in the impl (see the abstraction note in the header).          *)
(***************************************************************************)
CompareChecksums(p) ==
    \E q \in Remotes(p), frame \in FrameRange:
        /\ pendingChecksums[p][q][frame] # -1  \* q sent us a checksum for this frame
        /\ localChecksums[p][frame] # -1       \* We have a local checksum too
        /\ frame < lastConfirmedFrame[p]       \* Only compare confirmed frames
        /\ LET localChecksum == localChecksums[p][frame]
               remoteChecksum == pendingChecksums[p][q][frame]
           IN
            IF localChecksum = remoteChecksum
            THEN
                \* Checksums match - verify THIS pair only
                /\ syncHealth' = [syncHealth EXCEPT ![p][q] =
                    IF @ = "DesyncDetected" THEN "DesyncDetected" ELSE "InSync"]
                /\ lastVerifiedFrame' = [lastVerifiedFrame EXCEPT ![p][q] =
                    IF @ < frame THEN frame ELSE @]
                /\ pendingChecksums' = [pendingChecksums EXCEPT ![p][q][frame] = -1]
            ELSE
                \* Checksums differ - desync detected for THIS pair
                /\ syncHealth' = [syncHealth EXCEPT ![p][q] = "DesyncDetected"]
                /\ lastVerifiedFrame' = lastVerifiedFrame  \* Don't update on desync
                /\ pendingChecksums' = [pendingChecksums EXCEPT ![p][q][frame] = -1]
        /\ UNCHANGED <<currentFrame, lastConfirmedFrame, localChecksums,
                       lastSentChecksumFrame, divergedPeer, network>>

(***************************************************************************)
(* Action: Introduce desync (models a non-determinism bug in game logic)   *)
(* One-shot: a single nondeterministically-chosen peer diverges; the       *)
(* remaining peers keep agreeing with each other. Checksums the diverged   *)
(* peer computed BEFORE this step still match (faithful: divergence only   *)
(* affects frames simulated after the bug bites).                          *)
(***************************************************************************)
IntroduceDesync ==
    /\ divergedPeer = NoDivergence
    /\ \E d \in PEERS: divergedPeer' = d
    /\ UNCHANGED <<currentFrame, lastConfirmedFrame, localChecksums, lastSentChecksumFrame,
                   pendingChecksums, syncHealth, lastVerifiedFrame, network>>

(***************************************************************************)
(* Action: Done - all frames processed, allow termination                  *)
(***************************************************************************)
Done ==
    /\ \A p \in PEERS: currentFrame[p] = MAX_FRAME
    /\ \A p \in PEERS: lastConfirmedFrame[p] = MAX_FRAME
    /\ network = {}
    /\ UNCHANGED vars

(***************************************************************************)
(* Next State Relation                                                     *)
(***************************************************************************)
Next ==
    \/ \E p \in PEERS:
        \/ AdvanceFrame(p)
        \/ ConfirmFrame(p)
        \/ SendChecksum(p)
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
    /\ \A p \in PEERS: SF_vars(SendChecksum(p))

(***************************************************************************)
(* Specification                                                           *)
(***************************************************************************)
Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Derived aggregates (model the session-level query API)                  *)
(***************************************************************************)

\* Models P2PSession::is_synchronized(): every remote individually InSync.
\* (The impl filters to currently-CONNECTED remotes; this spec has no
\* disconnects, so all remotes count.)
IsSynchronized(p) ==
    \A q \in Remotes(p): syncHealth[p][q] = "InSync"

\* Models P2PSession::last_verified_frame(): the MIN over every remote's
\* individually-verified frame, undefined (-1 here, None in the impl) while
\* ANY remote is still unverified.
AggregateLastVerified(p) ==
    IF \A q \in Remotes(p): lastVerifiedFrame[p][q] >= 0
    THEN CHOOSE m \in {lastVerifiedFrame[p][q] : q \in Remotes(p)}:
            \A q \in Remotes(p): m <= lastVerifiedFrame[p][q]
    ELSE -1

(***************************************************************************)
(* Safety Properties                                                       *)
(***************************************************************************)

\* SAFE-DESYNC-1: No false positives, pair-precise.
\* A DesyncDetected verdict for pair (p,q) requires that p or q actually
\* diverged: two healthy peers never report a desync against each other.
\* (Strictly stronger than the old global "some divergence exists" form.)
NoFalsePositives ==
    \A p \in PEERS: \A q \in Remotes(p):
        (syncHealth[p][q] = "DesyncDetected") => (divergedPeer \in {p, q})

\* SAFE-DESYNC-2: DesyncDetected is terminal PER PAIR (once p has detected a
\* desync against q, no action -- in particular no match against a DIFFERENT
\* remote r -- may clear it). This is the property the old per-local keying
\* violated at N=3 (Session 27): it is the regression guard against
\* re-introducing any cross-pair clobbering.
DesyncIsTerminal ==
    [][\A p \in PEERS: \A q \in Remotes(p):
        syncHealth[p][q] = "DesyncDetected" => syncHealth'[p][q] = "DesyncDetected"
    ]_vars

\* SAFE-DESYNC-3: last_verified_frame is monotonically increasing per pair
VerifiedFrameMonotonic ==
    [][\A p \in PEERS: \A q \in Remotes(p):
        lastVerifiedFrame[p][q] <= lastVerifiedFrame'[p][q]
    ]_vars

\* SAFE-DESYNC-4: Comparison only happens on confirmed frames
\* (This is implicit in the action guard, not needed as explicit invariant)

\* SAFE-DESYNC-5 (F12-shaped): InSync for (p,q) requires a successful
\* comparison in pair (p,q) ITSELF -- lastVerifiedFrame[p][q] is only ever
\* set by a match of p's local checksum against a checksum q sent, so a pair
\* that never exchanged a matching checksum can never report InSync
\* (verification against q must not leak into the verdict for r).
InSyncImpliesMatchingChecksums ==
    \A p \in PEERS: \A q \in Remotes(p):
        (syncHealth[p][q] = "InSync") => (lastVerifiedFrame[p][q] >= 0)

\* SAFE-DESYNC-6 (F12 aggregate): the session-level is_synchronized() answer
\* requires EVERY pair to be individually verified -- one verified remote
\* must never make the aggregate true while another remote is unverified.
\* HONESTY NOTE: given InSyncImpliesMatchingChecksums and the definition of
\* IsSynchronized this is currently tautological -- it adds no verification
\* strength today. It is kept as a regression TRIPWIRE: it fails the moment
\* someone re-defines IsSynchronized (or weakens SAFE-DESYNC-5) in a way
\* that lets an unverified remote count as synchronized.
SynchronizedRequiresAllPairsVerified ==
    \A p \in PEERS:
        IsSynchronized(p) => \A q \in Remotes(p): lastVerifiedFrame[p][q] >= 0

\* SAFE-DESYNC-7: the aggregate last_verified_frame() is a lower bound of
\* every pair's verified frame (it is the MIN over connected remotes, so it
\* never reports a frame some remote has not actually verified).
\* HONESTY NOTE: like SAFE-DESYNC-6 this is tautological given the current
\* MIN definition of AggregateLastVerified -- a definition TRIPWIRE, not
\* added verification strength. It fails if the aggregate is ever changed
\* to MAX / latest / any non-lower-bound combination of the pair frames.
AggregateVerifiedFrameSound ==
    \A p \in PEERS:
        AggregateLastVerified(p) >= 0 =>
            \A q \in Remotes(p): lastVerifiedFrame[p][q] >= AggregateLastVerified(p)

(***************************************************************************)
(* Liveness Properties                                                     *)
(* (Defined for documentation; not checked -- see .cfg. As in the old N=2  *)
(* version, the premises can be invalidated by a later IntroduceDesync or  *)
(* by divergence after the last comparable frame, so they are unsound as   *)
(* stated and remain disabled.)                                            *)
(***************************************************************************)

\* LIVE-DESYNC-1: If no divergence and peers keep advancing, eventually all
\* pairs are InSync
EventuallyInSync ==
    (divergedPeer = NoDivergence /\ (\A p \in PEERS: currentFrame[p] >= CHECKSUM_INTERVAL))
        ~> (\A p \in PEERS: IsSynchronized(p))

\* LIVE-DESYNC-2: If a divergence is introduced, eventually some pair detects it
EventuallyDetected ==
    (divergedPeer # NoDivergence /\ \A p \in PEERS: lastConfirmedFrame[p] >= CHECKSUM_INTERVAL)
        ~> (\E p \in PEERS: \E q \in Remotes(p): syncHealth[p][q] = "DesyncDetected")

(***************************************************************************)
(* State Constraint for Model Checking                                     *)
(***************************************************************************)
(* Cardinality(network) <= 2 caps in-flight (sent, not yet received)       *)
(* messages at one N=3 broadcast. SOUNDNESS (why this cannot mask a        *)
(* violation of the checked properties): ReceiveChecksum(p) for a message  *)
(* m only deletes m from the network and copies its payload into           *)
(* pendingChecksums[p][m.from][m.frame]; it reads/writes no other state.   *)
(* It therefore commutes with every other action and never disables one:   *)
(* SendChecksum/AdvanceFrame/ConfirmFrame/IntroduceDesync guards read      *)
(* neither network nor pendingChecksums; CompareChecksums guards read      *)
(* pending entries only positively, and a receive only ADDS an entry --    *)
(* distinct (from,frame) per sender since lastSentChecksumFrame strictly   *)
(* increases, so no receive ever overwrites a pending entry; and Done's    *)
(* guard reads network = {}, but receives only DRAIN the network, so       *)
(* moving them earlier can only enable Done sooner -- and Done is          *)
(* UNCHANGED vars, a pure stutter, so it changes no checked property       *)
(* anyway. Hence any unconstrained trace can be reordered -- moving each   *)
(* receive to the earliest point after its send -- into a constrained      *)
(* trace with the IDENTICAL subsequence of Send/Compare/IntroduceDesync    *)
(* transitions and identical values of every variable the checked          *)
(* properties read at each of those transitions. The two                   *)
(* reordering-affected variables, network and pendingChecksums, differ     *)
(* (receive-early delivers entries SOONER) but harmlessly: no checked      *)
(* property reads either beyond TypeInvariant's typing clauses --          *)
(* per-message for network, so trivially preserved on any subset of sent   *)
(* messages, and preserved for pendingChecksums by every delivery, since   *)
(* the stored value is the message's Nat checksum -- and every pending     *)
(* entry a Compare consumes is written exactly once with the same message  *)
(* payload in both traces, so each Compare reads identical values and      *)
(* computes the identical result. Hence a violation reachable without      *)
(* the cap is also reachable with it. pendingChecksums (post-delivery      *)
(* state) is NOT capped: cross-pair comparison interleavings -- the        *)
(* Session 27 bug class -- are fully preserved.                            *)
StateConstraint ==
    /\ \A p \in PEERS: currentFrame[p] <= MAX_FRAME
    /\ \A p \in PEERS: lastConfirmedFrame[p] <= MAX_FRAME
    /\ Cardinality(network) <= 2

(***************************************************************************)
(* Invariants to Check                                                     *)
(***************************************************************************)
Invariants ==
    /\ TypeInvariant
    /\ NoFalsePositives
    /\ InSyncImpliesMatchingChecksums
    /\ SynchronizedRequiresAllPairsVerified
    /\ AggregateVerifiedFrameSound

=============================================================================
