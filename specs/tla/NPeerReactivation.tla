--------------------------- MODULE NPeerReactivation ---------------------------
(***************************************************************************)
(* TLA+ model of "Agreement C" -- the activation-frame agreement of        *)
(* N-peer mesh reconnection (Session 18 design, progress/                  *)
(* session-18-npeer-mesh-reconnection-design.md, sections 4.C / 5 / 8),    *)
(* extended (session 33) with the TWO-ATTEMPT retry space and the          *)
(* survivor ABORT-RESTORE (re-freeze) lifecycle the implementation ships.  *)
(*                                                                         *)
(* Two survivors is the minimal configuration that exercises the           *)
(* multi-party agreement, and the COMMITTED cfg checks exactly that        *)
(* ({S1, S2}, MaxFrame 3): the session-33 two-attempt extension multiplies *)
(* the per-survivor state space, so the larger pre-extension configuration *)
(* ({S1, S2, S3}, MaxFrame 4 — which the single-attempt model WAS checked  *)
(* at) no longer fits the CI budget. See the cfg header for the scope-down *)
(* rationale and the config-only path back to a larger out-of-CI run. The  *)
(* spec is parameterized over Survivors, so the invariants are STATED for  *)
(* any non-empty set of survivors — one coordinator K, the survivors, and  *)
(* one returning joiner J reopening a single dropped slot h — but only the *)
(* committed cfg's instance is exhaustively CHECKED.                       *)
(*                                                                         *)
(* What is modeled (the safety core of Agreement C):                       *)
(*   - Per-peer committed history for slot h: committedUpTo[p] is the       *)
(*     highest frame p has irrevocably confirmed; mode[p][f] in            *)
(*     {"frozen","real","none"} is the value-SOURCE p committed at frame f; *)
(*     and val[p][f] is the actual committed VALUE (bytes) at frame f.      *)
(*   - INDEPENDENT frozen values: frozenVal[p] is a distinct symbol per     *)
(*     surviving peer (NOT one global constant). A frozen commit stores     *)
(*     frozenVal[p]; a real commit stores the shared RealVal. Agreement is  *)
(*     checked over val (the bytes), so two survivors that both "froze" a   *)
(*     frame only AGREE when their frozenVal symbols are equal -- which is  *)
(*     exactly precondition P-A (Agreement A, clean freeze, section 4.A/5). *)
(*     This lets the spec DEMONSTRATE that P-A is NECESSARY (drop it and    *)
(*     Agreement breaks -- see NPeerReactivation_NoPA.cfg) rather than      *)
(*     silently assume it.                                                  *)
(*   - The keepalive-preserved CAP, modeled LIVE (section 4.C): survivors   *)
(*     race toward the open attempt's activation frame by committing frozen *)
(*     frames and are HELD one frame short of it while the paused           *)
(*     coordinator's keepalives keep it in their connected-min (capHeld).   *)
(*     CapCollapse models the wall-clock-timeout-during-pause hazard,       *)
(*     disabled while keepaliveServing (the protocol's guard).              *)
(*   - DEFERRED activation commits + the ABORT RESTORE (session-33          *)
(*     extension, closing review spec-gap 2). A survivor's reopen no longer *)
(*     commits frame F directly: it repositions the slot (reopened) and     *)
(*     acks; the COMMIT of F happens only on the attempt's JoinCommitted    *)
(*     lifecycle close (SurvivorCommitReal). On a JoinAborted close (or an  *)
(*     implied close), a reopened survivor RE-FREEZES: the reopen is        *)
(*     reverted with nothing committed at F, and the survivor then commits  *)
(*     F frozen exactly like every peer that never reopened. The invariant  *)
(*     RealOnlyInCommittedAttempt pins the consequence: no peer ever holds  *)
(*     a REAL commit at an attempt's activation frame unless that attempt   *)
(*     committed. (The previous model committed F real at reopen and kept   *)
(*     it across an abort -- the implementation deliberately restores       *)
(*     instead, because a kept real-at-F commit would diverge from every    *)
(*     survivor that froze F after the abort; with the restore modeled,     *)
(*     Agreement itself now checks that convergence.)                       *)
(*   - A SECOND same-coordinator attempt (session-33 extension, closing     *)
(*     review spec-gap 1 / the Finding-2 space). After attempt 1 aborts,    *)
(*     the coordinator advances one frame (committing F1 frozen -- the R3   *)
(*     guard) and re-serves at F2 = F1 + 1 > F1. Stale attempt-1 directives *)
(*     remain in flight and deliverable DURING attempt 2, and the attempt-1 *)
(*     JoinAborted responder is destroyed when attempt 2 opens (no further  *)
(*     retransmits; in-flight copies may still land or drop). The survivor  *)
(*     directive rules under test are exactly the implementation's:        *)
(*       (leg 1) a pre-reopen pending attempt is superseded only by a       *)
(*               STRICTLY NEWER same-coordinator attempt (frame order);     *)
(*               a stale older directive is consumed and ignored;          *)
(*       (leg 2) a directive at or below the survivor's CLOSED-attempt      *)
(*               high-water (closedHW) is rejected (NoStaleReopen pins      *)
(*               this: no pending attempt ever sits at/below the high-      *)
(*               water);                                                    *)
(*       (leg 3) a REOPENED pending attempt is closed by a strictly newer   *)
(*               same-coordinator directive (implied close: the directive   *)
(*               proves the old attempt concluded; with no commit evidence  *)
(*               -- always the case here, attempt 2 exists only after an    *)
(*               abort -- the close applies the abort restore) and the new  *)
(*               attempt is accepted.                                       *)
(*     Without leg 3 a survivor whose JoinAborted{F1} was lost would wedge  *)
(*     reopened-at-F1 forever and attempt 2 could never gather its ack;    *)
(*     with it, attempt 2 commits (liveness EventuallyResolved + the        *)
(*     joined2 terminal exercises the heal end to end).                     *)
(*   - The LATE-APPLY abort lifecycle for the joiner (section 5), per       *)
(*     attempt: buffer -> commit only on JoinCommitted / discard on         *)
(*     JoinAborted, so the joiner is never real-at-F of an aborted attempt. *)
(*                                                                         *)
(* Properties verified (section 5 + session-33 extensions):                *)
(*   - Agreement (S1): any two peers that committed a frame committed the   *)
(*     same VALUE there -- now including post-abort frozen commits of F1    *)
(*     and the attempt-2 space.                                             *)
(*   - NoConfirmedRewrite (S2): committed history never reverts.            *)
(*   - NoSplitBrainOnAbort (L1): no aborted attempt has the joiner real at  *)
(*     its F while a survivor is frozen at it.                              *)
(*   - RealOnlyInCommittedAttempt: an attempt's activation frame is REAL-   *)
(*     committed only if that attempt committed (the abort-restore pin).    *)
(*   - NoStaleReopen: no pending attempt at/below the closed high-water     *)
(*     (the stale-directive guard pin).                                     *)
(*   - Liveness: the protocol eventually reaches joined, joined2, or the    *)
(*     terminal aborted2.                                                   *)
(*                                                                         *)
(* What is NOT modeled (honest scope; tracked residuals):                  *)
(*   - Endpoint death (the joiner-endpoint-death close and the             *)
(*     coordinator-death window) -- the implementation's local close path   *)
(*     has no message-level counterpart here. A survivor wedged             *)
(*     reopened-at-F2 when attempt 2 itself aborts therefore stays pending  *)
(*     in the terminal aborted2 state if the JoinAborted{F2} copies are     *)
(*     all lost; the implementation escapes via the joiner-endpoint death   *)
(*     close (chunk-N4 teardown contract).                                  *)
(*   - The commit-evidence arm of the implied/local closes (the            *)
(*     confirmed-history >= F read plus the gossip freeze-frame >= F leg):  *)
(*     unreachable in this phase structure because a second attempt for    *)
(*     the same slot only follows an ABORT; the committed-then-re-drop     *)
(*     shape needs the N5 re-drop model.                                    *)
(*   - Gossip stickiness / in-flight connect-status reordering (the         *)
(*     reactivation-floor merge guard) and survivor speculation past F     *)
(*     (the forced re-simulation) -- value-stream level concerns below      *)
(*     this model's commit abstraction (the freeze-CONVERGENCE family, by   *)
(*     contrast, IS expressible at this level -- see the next item).        *)
(*   - Per-survivor pre-attempt freeze frames: f0 is implicitly UNIFORM     *)
(*     across survivors here, so the session-33 round-5 freeze-convergence  *)
(*     gates (acceptance/serve-open deferral; owed re-adjust defer/abort)   *)
(*     are checked VACUOUSLY by this model. With per-survivor f0 and the    *)
(*     convergence re-adjust modeled, Agreement over val would express      *)
(*     exactly that divergence; the extension is the tracked spec-budget    *)
(*     item (session-33 residual 10).                                       *)
(***************************************************************************)

EXTENDS Naturals, FiniteSets, TLC

CONSTANTS
    Survivors,      \* set of surviving peers, e.g. {"S1", "S2", "S3"}
    K,              \* the coordinator / snapshot server
    J,              \* the returning joiner
    MaxFrame,       \* highest frame any peer may confirm (small bound)
    RealVal,        \* the (shared, agreed) real value-source symbol at frames >= F
    NoMode,         \* sentinel for "no value-source committed at this frame"
    NoVal,          \* sentinel for "no value committed at this frame"
    AssumePA        \* TRUE to pin precondition P-A in Init; FALSE for the demo

ASSUME Survivors # {}
ASSUME K \notin Survivors
ASSUME J \notin Survivors /\ J # K
ASSUME MaxFrame \in Nat /\ MaxFrame > 2
ASSUME NoMode \notin {"frozen", "real"}
ASSUME AssumePA \in BOOLEAN

MeshPeers == Survivors \union {K}

Frames  == 0..MaxFrame
Modes   == {"frozen", "real", NoMode}

(***************************************************************************)
(* Attempts are numbered; 0 is the "none" sentinel. R3 (the next-serve      *)
(* guard) makes activation frames strictly monotone across a coordinator's *)
(* attempts, so attempt order IS frame order: AttF(1) = F1 = L + 1 and     *)
(* AttF(2) = F2 = L + 2.                                                    *)
(***************************************************************************)
Attempts == {1, 2}

VARIABLES
    phase,              \* coordinator lifecycle phase (two-attempt chain)
    L,                  \* coordinator's sent / last-saved frame at attempt 1
    committedUpTo,      \* [MeshPeers -> Frames]: highest frame each mesh peer confirmed
    mode,               \* [MeshPeers -> [Frames -> Modes]]: committed value-SOURCE per frame
    val,                \* [MeshPeers -> [Frames -> Vals]]:  committed VALUE (bytes) per frame
    frozenVal,          \* [Survivors -> Nat]: INDEPENDENT frozen value symbol per survivor
    capHeld,            \* [Survivors -> BOOLEAN]: K is in survivor s's connected-min
    keepaliveServing,   \* TRUE while a paused coordinator is serving keepalives
    pendingA,           \* [Survivors -> {0} \union Attempts]: the survivor's pending attempt
    reopened,           \* [Survivors -> BOOLEAN]: the pending attempt has reopened the slot
    closedHW,           \* [Survivors -> {0} \union Attempts]: highest CLOSED attempt (high-water)
    dirInflight,        \* [Survivors -> [Attempts -> BOOLEAN]]: directive copies in flight
    abInflight,         \* [Survivors -> [Attempts -> BOOLEAN]]: JoinAborted copies in flight
    acked,              \* [Survivors -> [Attempts -> BOOLEAN]]: coordinator holds the reopen-ack
    kReopened,          \* TRUE once the coordinator reopened its own slot (commit)
    joinerBuffered,     \* {0} \union Attempts: attempt whose snapshot the joiner buffered
    joinerCommittedF,   \* TRUE once the joiner irrevocably committed real at its attempt's F
    joinerMode,         \* [Frames -> Modes]
    joinerVal           \* [Frames -> Vals]

vars == <<phase, L, committedUpTo, mode, val, frozenVal, capHeld,
          keepaliveServing, pendingA, reopened, closedHW, dirInflight,
          abInflight, acked, kReopened, joinerBuffered, joinerCommittedF,
          joinerMode, joinerVal>>

\* Attempt 1: paused -> joined (terminal) | aborted -> paused2 ->
\* joined2 (terminal) | aborted2 (terminal).
Phases == {"paused", "joined", "aborted", "paused2", "joined2", "aborted2"}

AttF(a) == L + a                            \* F1 = L + 1, F2 = L + 2 (R3 order)

\* The attempt currently OPEN on the coordinator in each paused phase.
OpenAttempt == IF phase = "paused" THEN 1 ELSE 2

FrozenValDomain == 0..1                     \* small independent symbol domain
Vals == FrozenValDomain \union {RealVal, NoVal}

FrozenAgreement ==
    \A s1, s2 \in Survivors: frozenVal[s1] = frozenVal[s2]

(***************************************************************************)
(* Type invariant.                                                         *)
(***************************************************************************)
TypeInvariant ==
    /\ phase \in Phases
    /\ L \in Frames
    /\ committedUpTo \in [MeshPeers -> Frames]
    /\ mode \in [MeshPeers -> [Frames -> Modes]]
    /\ val \in [MeshPeers -> [Frames -> Vals]]
    /\ frozenVal \in [Survivors -> FrozenValDomain]
    /\ capHeld \in [Survivors -> BOOLEAN]
    /\ keepaliveServing \in BOOLEAN
    /\ pendingA \in [Survivors -> {0} \union Attempts]
    /\ reopened \in [Survivors -> BOOLEAN]
    /\ closedHW \in [Survivors -> {0} \union Attempts]
    /\ dirInflight \in [Survivors -> [Attempts -> BOOLEAN]]
    /\ abInflight \in [Survivors -> [Attempts -> BOOLEAN]]
    /\ acked \in [Survivors -> [Attempts -> BOOLEAN]]
    /\ kReopened \in BOOLEAN
    /\ joinerBuffered \in {0} \union Attempts
    /\ joinerCommittedF \in BOOLEAN
    /\ joinerMode \in [Frames -> Modes]
    /\ joinerVal \in [Frames -> Vals]

(***************************************************************************)
(* Init: the mesh starts paused inside attempt 1 (F1 = L + 1 chosen, the   *)
(* directive in flight to every survivor); each survivor starts confirmed  *)
(* anywhere in 0..L (it may lag the cap) and races up. L leaves room for    *)
(* the retry attempt's F2 = L + 2 <= MaxFrame.                              *)
(***************************************************************************)
SomeSurvivor == CHOOSE s \in Survivors: TRUE

PeerFrozenSym(p) == IF p \in Survivors THEN frozenVal[p] ELSE frozenVal[SomeSurvivor]

InitMode(p, c) == [f \in Frames |-> IF f <= c THEN "frozen" ELSE NoMode]

InitVal(p, c) ==
    [f \in Frames |-> IF f <= c THEN PeerFrozenSym(p) ELSE NoVal]

Init ==
    /\ L \in 1..(MaxFrame - 2)             \* room for F2 = L + 2 <= MaxFrame
    /\ phase = "paused"
    /\ frozenVal \in [Survivors -> FrozenValDomain]
    /\ AssumePA => FrozenAgreement         \* P-A: all survivors share one frozen value
    \* Sound state-space reduction (a representative pin, NOT a new semantic
    \* assumption). Under P-A every survivor holds the SAME frozen value, and the
    \* two symbols in FrozenValDomain (0 and 1) are interchangeable -- they are
    \* only ever compared by equality in Agreement, never used arithmetically --
    \* so the all-0 and all-1 initial assignments are ISOMORPHIC: the all-1 half
    \* of the reachable space is a redundant mirror of the all-0 half. Fixing the
    \* shared value to the 0 representative (FrozenAgreement above already forces
    \* equality, so this pins all survivors) halves the reachable space
    \* (582,112 -> 291,056 distinct states) with ZERO loss of coverage. Unlike
    \* SYMMETRY Permutations(Survivors) this is SOUND under LIVENESS checking: it
    \* adds no permutation equivalence classes and no CHOOSE-over-a-symmetry-set
    \* hazard (cf. SomeSurvivor / PeerFrozenSym), so EventuallyResolved is still
    \* checked exhaustively. Guarded by AssumePA, so the NPeerReactivation_NoPA
    \* demonstration (AssumePA = FALSE) is untouched and still exhibits its
    \* Agreement counterexample (where survivors hold DIFFERENT frozen values).
    /\ AssumePA => frozenVal[SomeSurvivor] = 0
    /\ \E start \in [Survivors -> Frames]:
         /\ \A s \in Survivors: start[s] <= L
         /\ committedUpTo = [p \in MeshPeers |-> IF p \in Survivors THEN start[p] ELSE L]
         /\ mode = [p \in MeshPeers |->
                      InitMode(p, IF p \in Survivors THEN start[p] ELSE L)]
         /\ val  = [p \in MeshPeers |->
                      InitVal(p, IF p \in Survivors THEN start[p] ELSE L)]
    /\ capHeld = [s \in Survivors |-> TRUE]
    /\ keepaliveServing = TRUE
    /\ pendingA = [s \in Survivors |-> 0]
    /\ reopened = [s \in Survivors |-> FALSE]
    /\ closedHW = [s \in Survivors |-> 0]
    /\ dirInflight = [s \in Survivors |-> [a \in Attempts |-> a = 1]]
    /\ abInflight = [s \in Survivors |-> [a \in Attempts |-> FALSE]]
    /\ acked = [s \in Survivors |-> [a \in Attempts |-> FALSE]]
    /\ kReopened = FALSE
    /\ joinerBuffered = 0
    /\ joinerCommittedF = FALSE
    /\ joinerMode = [f \in Frames |-> NoMode]
    /\ joinerVal = [f \in Frames |-> NoVal]

(***************************************************************************)
(* SurvivorAdvanceFrozen -- a survivor confirms one more frozen frame.      *)
(*                                                                         *)
(* Below the OPEN attempt's activation frame this is always allowed (the   *)
(* survivor races up to the cap boundary; between the attempts -- the      *)
(* "aborted" window -- the boundary is already attempt 2's, because the    *)
(* un-paused coordinator's own advance to F1 re-pins the live min there).  *)
(* Committing the open activation frame itself frozen requires the cap to  *)
(* have COLLAPSED (~capHeld). A REOPENED survivor does not frozen-advance:  *)
(* its queue is repositioned at its pending attempt's frame (this is also  *)
(* what makes a stale wedged reopen a liveness stall, exactly as in the    *)
(* implementation, until the implied close heals it).                       *)
(* In terminal phases the mesh advance is irrelevant to the properties and *)
(* is left enabled below F-bounds for uniformity.                           *)
(***************************************************************************)
CapBoundary ==
    CASE phase = "paused"  -> AttF(1)
      [] phase = "joined"  -> AttF(1)
      [] OTHER             -> AttF(2)

SurvivorAdvanceFrozen(s) ==
    /\ s \in Survivors
    /\ phase \in {"paused", "aborted", "paused2", "aborted2"}
    /\ committedUpTo[s] < MaxFrame
    /\ ~reopened[s]
    /\ (committedUpTo[s] + 1 >= CapBoundary => ~capHeld[s])
    /\ committedUpTo' = [committedUpTo EXCEPT ![s] = committedUpTo[s] + 1]
    /\ mode' = [mode EXCEPT ![s][committedUpTo[s] + 1] = "frozen"]
    /\ val'  = [val  EXCEPT ![s][committedUpTo[s] + 1] = frozenVal[s]]
    /\ UNCHANGED <<phase, L, frozenVal, capHeld, keepaliveServing, pendingA,
                   reopened, closedHW, dirInflight, abInflight, acked,
                   kReopened, joinerBuffered, joinerCommittedF, joinerMode,
                   joinerVal>>

(***************************************************************************)
(* CapCollapse -- the cap-collapse hazard (design section 4.C). Disabled    *)
(* while the paused coordinator serves keepalives. (See the original        *)
(* rationale; unchanged by the session-33 extension, and applicable in      *)
(* either paused phase.)                                                    *)
(***************************************************************************)
CapCollapse(s) ==
    /\ s \in Survivors
    /\ phase \in {"paused", "paused2"}
    /\ capHeld[s]
    /\ ~keepaliveServing
    /\ capHeld' = [capHeld EXCEPT ![s] = FALSE]
    /\ UNCHANGED <<phase, L, committedUpTo, mode, val, frozenVal,
                   keepaliveServing, pendingA, reopened, closedHW, dirInflight,
                   abInflight, acked, kReopened, joinerBuffered,
                   joinerCommittedF, joinerMode, joinerVal>>

(***************************************************************************)
(* DeliverDirective -- a ReactivateSlot{h, AttF(a)} copy reaches survivor   *)
(* s. The survivor applies the implementation's fail-closed rules:          *)
(*                                                                         *)
(*   - fresh (no pending): accept iff a > closedHW[s] (leg 2, the closed-   *)
(*     attempt high-water guard) and AttF(a) > committedUpTo[s] (F-sanity); *)
(*     otherwise consume-and-ignore.                                        *)
(*   - pending pre-reopen: supersede iff a > pendingA[s] (leg 1, strict     *)
(*     frame order -- a stale older duplicate is consumed and ignored);     *)
(*     same-attempt duplicates are no-ops.                                  *)
(*   - pending REOPENED: a strictly newer directive IMPLIES the pending     *)
(*     attempt concluded (one-join-at-a-time + R3); with no commit          *)
(*     evidence -- structurally the case here -- the survivor applies the   *)
(*     abort RESTORE (reopen reverted, nothing was committed at the old F)  *)
(*     and accepts the new attempt (leg 3). Older/duplicate directives are  *)
(*     consumed and ignored (a reopened attempt is otherwise closed only    *)
(*     by its own lifecycle messages).                                      *)
(***************************************************************************)
AcceptableFresh(s, a) == a > closedHW[s] /\ AttF(a) > committedUpTo[s]

DeliverDirective(s, a) ==
    /\ s \in Survivors
    /\ a \in Attempts
    /\ dirInflight[s][a]
    /\ dirInflight' = [dirInflight EXCEPT ![s][a] = FALSE]
    /\ IF pendingA[s] = 0 /\ AcceptableFresh(s, a)
         THEN /\ pendingA' = [pendingA EXCEPT ![s] = a]
              /\ UNCHANGED <<reopened, closedHW>>
       ELSE IF pendingA[s] # 0 /\ a > pendingA[s] /\ ~reopened[s]
         THEN \* leg 1: strictly-newer same-coordinator retry supersedes
              /\ pendingA' = [pendingA EXCEPT ![s] = a]
              /\ UNCHANGED <<reopened, closedHW>>
       ELSE IF pendingA[s] # 0 /\ a > pendingA[s] /\ reopened[s]
         THEN \* leg 3: implied close (abort restore) + accept the new attempt
              /\ closedHW' = [closedHW EXCEPT ![s] =
                                IF pendingA[s] > closedHW[s] THEN pendingA[s]
                                ELSE closedHW[s]]
              /\ reopened' = [reopened EXCEPT ![s] = FALSE]
              /\ pendingA' = [pendingA EXCEPT ![s] = a]
       ELSE \* stale / duplicate / unacceptable: consumed, no state change
            UNCHANGED <<pendingA, reopened, closedHW>>
    /\ UNCHANGED <<phase, L, committedUpTo, mode, val, frozenVal, capHeld,
                   keepaliveServing, abInflight, acked, kReopened,
                   joinerBuffered, joinerCommittedF, joinerMode, joinerVal>>

(***************************************************************************)
(* DropDirective / RetransmitDirective -- lossy delivery. The coordinator   *)
(* retransmits only its OPEN attempt's directive to survivors that have not *)
(* acked it; stale attempt-1 copies are never re-created once attempt 2 is  *)
(* open (matching the implementation's per-serve fan-out), but copies       *)
(* already in flight remain deliverable at any time -- exactly the          *)
(* Finding-2 stale-directive space.                                         *)
(***************************************************************************)
DropDirective(s, a) ==
    /\ s \in Survivors
    /\ a \in Attempts
    /\ dirInflight[s][a]
    /\ dirInflight' = [dirInflight EXCEPT ![s][a] = FALSE]
    /\ UNCHANGED <<phase, L, committedUpTo, mode, val, frozenVal, capHeld,
                   keepaliveServing, pendingA, reopened, closedHW, abInflight,
                   acked, kReopened, joinerBuffered, joinerCommittedF,
                   joinerMode, joinerVal>>

RetransmitDirective(s) ==
    /\ s \in Survivors
    /\ phase \in {"paused", "paused2"}
    /\ ~acked[s][OpenAttempt]
    /\ ~dirInflight[s][OpenAttempt]
    /\ dirInflight' = [dirInflight EXCEPT ![s][OpenAttempt] = TRUE]
    /\ UNCHANGED <<phase, L, committedUpTo, mode, val, frozenVal, capHeld,
                   keepaliveServing, pendingA, reopened, closedHW, abInflight,
                   acked, kReopened, joinerBuffered, joinerCommittedF,
                   joinerMode, joinerVal>>

(***************************************************************************)
(* SurvivorReopen -- the survivor's pending attempt goes live: once it has  *)
(* caught up to the attempt's cap boundary (committedUpTo = AttF(a) - 1,    *)
(* gap-free history) it repositions the slot at AttF(a) and acks. NOTHING   *)
(* is committed at AttF(a) yet -- the commit is deferred to the lifecycle   *)
(* close (session-33 extension; this is what makes the abort restore a      *)
(* sound revert).                                                           *)
(*                                                                         *)
(* The ack only reaches the coordinator's CURRENT attempt: a stale ack for  *)
(* an older attempt is discriminated and dropped by the coordinator         *)
(* (modeled by acked being per-attempt and the commit barrier reading only  *)
(* the open attempt's column).                                              *)
(***************************************************************************)
SurvivorReopen(s) ==
    /\ s \in Survivors
    /\ pendingA[s] # 0
    /\ ~reopened[s]
    /\ phase \in {"paused", "aborted", "paused2"}
    /\ committedUpTo[s] = AttF(pendingA[s]) - 1
    /\ reopened' = [reopened EXCEPT ![s] = TRUE]
    /\ acked' = [acked EXCEPT ![s][pendingA[s]] = TRUE]
    /\ UNCHANGED <<phase, L, committedUpTo, mode, val, frozenVal, capHeld,
                   keepaliveServing, pendingA, closedHW, dirInflight,
                   abInflight, kReopened, joinerBuffered, joinerCommittedF,
                   joinerMode, joinerVal>>

(***************************************************************************)
(* JoinerBuffer -- the joiner receives and BUFFERS the open attempt's       *)
(* snapshot (late-apply lifecycle; per attempt).                            *)
(***************************************************************************)
JoinerBuffer ==
    /\ phase \in {"paused", "paused2"}
    /\ joinerBuffered = 0
    /\ joinerBuffered' = OpenAttempt
    /\ UNCHANGED <<phase, L, committedUpTo, mode, val, frozenVal, capHeld,
                   keepaliveServing, pendingA, reopened, closedHW, dirInflight,
                   abInflight, acked, kReopened, joinerCommittedF, joinerMode,
                   joinerVal>>

(***************************************************************************)
(* UnpauseAndCommit -- ack-gated un-pause (success path of the open         *)
(* attempt). The coordinator commits the activation frame REAL on itself   *)
(* and enters the joined phase; the joiner and the survivors then commit    *)
(* via the lifecycle actions below.                                         *)
(***************************************************************************)
UnpauseAndCommit ==
    /\ phase \in {"paused", "paused2"}
    /\ \A s \in Survivors: acked[s][OpenAttempt]
    /\ joinerBuffered = OpenAttempt
    /\ phase' = IF phase = "paused" THEN "joined" ELSE "joined2"
    /\ keepaliveServing' = FALSE
    /\ kReopened' = TRUE
    /\ committedUpTo' = [committedUpTo EXCEPT ![K] = AttF(OpenAttempt)]
    /\ mode' = [mode EXCEPT ![K][AttF(OpenAttempt)] = "real"]
    /\ val'  = [val  EXCEPT ![K][AttF(OpenAttempt)] = RealVal]
    /\ UNCHANGED <<L, frozenVal, capHeld, pendingA, reopened, closedHW,
                   dirInflight, abInflight, acked, joinerBuffered,
                   joinerCommittedF, joinerMode, joinerVal>>

(***************************************************************************)
(* SurvivorCommitReal -- the attempt's JoinCommitted lifecycle close        *)
(* reaches a reopened survivor: it irrevocably commits the activation       *)
(* frame REAL and closes the pending attempt (recording the high-water).    *)
(* Only reachable in the matching joined phase, whose barrier required      *)
(* every survivor's ack -- so the pending attempt here is always the        *)
(* committed one.                                                           *)
(***************************************************************************)
CommittedAttempt == IF phase = "joined" THEN 1 ELSE 2

SurvivorCommitReal(s) ==
    /\ s \in Survivors
    /\ phase \in {"joined", "joined2"}
    /\ pendingA[s] = CommittedAttempt
    /\ reopened[s]
    /\ committedUpTo' = [committedUpTo EXCEPT ![s] = AttF(pendingA[s])]
    /\ mode' = [mode EXCEPT ![s][AttF(pendingA[s])] = "real"]
    /\ val'  = [val  EXCEPT ![s][AttF(pendingA[s])] = RealVal]
    /\ closedHW' = [closedHW EXCEPT ![s] =
                      IF pendingA[s] > closedHW[s] THEN pendingA[s]
                      ELSE closedHW[s]]
    /\ pendingA' = [pendingA EXCEPT ![s] = 0]
    /\ reopened' = [reopened EXCEPT ![s] = FALSE]
    /\ UNCHANGED <<phase, L, frozenVal, capHeld, keepaliveServing,
                   dirInflight, abInflight, acked, kReopened, joinerBuffered,
                   joinerCommittedF, joinerMode, joinerVal>>

(***************************************************************************)
(* Timeout -- the open attempt's Phase-4 serve timeout: the coordinator     *)
(* aborts and fans JoinAborted{AttF(a)} out to every survivor.              *)
(***************************************************************************)
Timeout ==
    /\ phase \in {"paused", "paused2"}
    /\ ~(\A s \in Survivors: acked[s][OpenAttempt])
    /\ phase' = IF phase = "paused" THEN "aborted" ELSE "aborted2"
    /\ keepaliveServing' = FALSE
    /\ abInflight' = [s \in Survivors |->
                        [a \in Attempts |->
                           IF a = OpenAttempt THEN TRUE ELSE abInflight[s][a]]]
    /\ UNCHANGED <<L, committedUpTo, mode, val, frozenVal, capHeld, pendingA,
                   reopened, closedHW, dirInflight, acked, kReopened,
                   joinerBuffered, joinerCommittedF, joinerMode, joinerVal>>

(***************************************************************************)
(* DeliverAbort / DropAbort / RetransmitAbort -- the JoinAborted lifecycle. *)
(* A matching close clears the pending attempt: pre-reopen it is a pure     *)
(* clear (the slot never left the frozen shape); post-reopen it applies the *)
(* RESTORE (reopen reverted; nothing was committed at the attempt's F, so   *)
(* the survivor resumes frozen-advancing and commits that frame frozen      *)
(* exactly like every peer that never reopened -- the implementation's      *)
(* refreeze_with_value + status restore + forced re-simulation collapse to  *)
(* this revert at the commit abstraction). Mismatched copies are consumed   *)
(* and ignored. The high-water is recorded on every close (leg 2).          *)
(*                                                                         *)
(* The attempt-1 abort responder is DESTROYED when attempt 2 opens:         *)
(* RetransmitAbort re-sends only while the matching aborted phase is        *)
(* current. In-flight copies remain deliverable (or droppable) at any time. *)
(***************************************************************************)
DeliverAbort(s, a) ==
    /\ s \in Survivors
    /\ a \in Attempts
    /\ abInflight[s][a]
    /\ abInflight' = [abInflight EXCEPT ![s][a] = FALSE]
    /\ IF pendingA[s] = a
         THEN /\ closedHW' = [closedHW EXCEPT ![s] =
                                IF a > closedHW[s] THEN a ELSE closedHW[s]]
              /\ pendingA' = [pendingA EXCEPT ![s] = 0]
              /\ reopened' = [reopened EXCEPT ![s] = FALSE]
       ELSE UNCHANGED <<pendingA, reopened, closedHW>>
    /\ UNCHANGED <<phase, L, committedUpTo, mode, val, frozenVal, capHeld,
                   keepaliveServing, dirInflight, acked, kReopened,
                   joinerBuffered, joinerCommittedF, joinerMode, joinerVal>>

DropAbort(s, a) ==
    /\ s \in Survivors
    /\ a \in Attempts
    /\ abInflight[s][a]
    /\ abInflight' = [abInflight EXCEPT ![s][a] = FALSE]
    /\ UNCHANGED <<phase, L, committedUpTo, mode, val, frozenVal, capHeld,
                   keepaliveServing, pendingA, reopened, closedHW, dirInflight,
                   acked, kReopened, joinerBuffered, joinerCommittedF,
                   joinerMode, joinerVal>>

RetransmitAbort(s) ==
    /\ s \in Survivors
    /\ phase \in {"aborted", "aborted2"}
    /\ LET a == IF phase = "aborted" THEN 1 ELSE 2 IN
         /\ pendingA[s] = a                  \* still un-closed (the re-ack ping)
         /\ ~abInflight[s][a]
         /\ abInflight' = [abInflight EXCEPT ![s][a] = TRUE]
    /\ UNCHANGED <<phase, L, committedUpTo, mode, val, frozenVal, capHeld,
                   keepaliveServing, pendingA, reopened, closedHW, dirInflight,
                   acked, kReopened, joinerBuffered, joinerCommittedF,
                   joinerMode, joinerVal>>

(***************************************************************************)
(* JoinerCommit -- the joiner applies its buffered snapshot on the open     *)
(* attempt's JoinCommitted: bridge frame AttF - 1 (carried frozen value) +  *)
(* real at AttF.                                                            *)
(***************************************************************************)
JoinerCommit ==
    /\ phase \in {"joined", "joined2"}
    /\ joinerBuffered = CommittedAttempt
    /\ ~joinerCommittedF
    /\ joinerCommittedF' = TRUE
    /\ joinerMode' = [f \in Frames |->
                        IF f = AttF(CommittedAttempt) - 1 THEN "frozen"
                        ELSE IF f = AttF(CommittedAttempt) THEN "real"
                        ELSE joinerMode[f]]
    /\ joinerVal'  = [f \in Frames |->
                        IF f = AttF(CommittedAttempt) - 1
                          THEN val[K][AttF(CommittedAttempt) - 1]
                        ELSE IF f = AttF(CommittedAttempt) THEN RealVal
                        ELSE joinerVal[f]]
    /\ UNCHANGED <<phase, L, committedUpTo, mode, val, frozenVal, capHeld,
                   keepaliveServing, pendingA, reopened, closedHW, dirInflight,
                   abInflight, acked, kReopened, joinerBuffered>>

(***************************************************************************)
(* JoinerAbortDiscard -- the joiner discards its buffered (aborted-attempt) *)
(* snapshot; it never committed real at that attempt's F.                   *)
(***************************************************************************)
JoinerAbortDiscard ==
    /\ phase \in {"aborted", "aborted2"}
    /\ joinerBuffered # 0
    /\ joinerBuffered' = 0
    /\ UNCHANGED <<phase, L, committedUpTo, mode, val, frozenVal, capHeld,
                   keepaliveServing, pendingA, reopened, closedHW, dirInflight,
                   abInflight, acked, kReopened, joinerCommittedF, joinerMode,
                   joinerVal>>

(***************************************************************************)
(* RetryOpen -- the coordinator opens attempt 2 after attempt 1 aborted     *)
(* (session-33 extension). The R3 next-serve guard requires it to have      *)
(* advanced past the aborted attempt's snapshot frame first: it commits     *)
(* F1 = L + 1 FROZEN (the slot is still frozen on the un-paused             *)
(* coordinator) and re-pauses at L2 = F1, choosing F2 = L2 + 1 = L + 2 --   *)
(* strictly newer, making every stale attempt-1 message discriminable.      *)
(* Opening DESTROYS the attempt-1 abort responder (RetransmitAbort above    *)
(* only serves the current aborted phase) and fans the attempt-2 directive  *)
(* out. The joiner must have discarded its aborted buffer (it re-requests). *)
(***************************************************************************)
RetryOpen ==
    /\ phase = "aborted"
    /\ joinerBuffered = 0
    /\ phase' = "paused2"
    /\ keepaliveServing' = TRUE
    /\ committedUpTo' = [committedUpTo EXCEPT ![K] = AttF(1)]
    /\ mode' = [mode EXCEPT ![K][AttF(1)] = "frozen"]
    /\ val'  = [val  EXCEPT ![K][AttF(1)] = PeerFrozenSym(K)]
    /\ dirInflight' = [s \in Survivors |->
                         [a \in Attempts |->
                            IF a = 2 THEN TRUE ELSE dirInflight[s][a]]]
    /\ UNCHANGED <<L, frozenVal, capHeld, pendingA, reopened, closedHW,
                   abInflight, acked, kReopened, joinerBuffered,
                   joinerCommittedF, joinerMode, joinerVal>>

(***************************************************************************)
(* Terminal stutter -- joined/joined2 once the joiner committed, aborted2   *)
(* once the joiner discarded. ("aborted" is not terminal: RetryOpen         *)
(* continues the run.)                                                      *)
(***************************************************************************)
TerminalStutter ==
    /\ \/ (phase \in {"joined", "joined2"} /\ joinerCommittedF)
       \/ (phase = "aborted2" /\ joinerBuffered = 0)
    /\ UNCHANGED vars

Next ==
    \/ \E s \in Survivors: SurvivorAdvanceFrozen(s)
    \/ \E s \in Survivors: CapCollapse(s)
    \/ \E s \in Survivors, a \in Attempts: DeliverDirective(s, a)
    \/ \E s \in Survivors, a \in Attempts: DropDirective(s, a)
    \/ \E s \in Survivors: RetransmitDirective(s)
    \/ \E s \in Survivors: SurvivorReopen(s)
    \/ \E s \in Survivors: SurvivorCommitReal(s)
    \/ \E s \in Survivors, a \in Attempts: DeliverAbort(s, a)
    \/ \E s \in Survivors, a \in Attempts: DropAbort(s, a)
    \/ \E s \in Survivors: RetransmitAbort(s)
    \/ JoinerBuffer
    \/ UnpauseAndCommit
    \/ JoinerCommit
    \/ Timeout
    \/ JoinerAbortDiscard
    \/ RetryOpen
    \/ TerminalStutter

(***************************************************************************)
(* Fairness: deliveries, catch-up, reopen/commit/close steps, and the       *)
(* coordinator's decisions are weakly fair; loss (the Drop actions) and the *)
(* hazard (CapCollapse) are never forced.                                   *)
(***************************************************************************)
Fairness ==
    /\ \A s \in Survivors: WF_vars(SurvivorAdvanceFrozen(s))
    /\ \A s \in Survivors: \A a \in Attempts: WF_vars(DeliverDirective(s, a))
    /\ \A s \in Survivors: WF_vars(RetransmitDirective(s))
    /\ \A s \in Survivors: WF_vars(SurvivorReopen(s))
    /\ \A s \in Survivors: WF_vars(SurvivorCommitReal(s))
    /\ \A s \in Survivors: \A a \in Attempts: WF_vars(DeliverAbort(s, a))
    /\ \A s \in Survivors: WF_vars(RetransmitAbort(s))
    /\ WF_vars(JoinerBuffer)
    /\ WF_vars(UnpauseAndCommit)
    /\ WF_vars(JoinerCommit)
    /\ WF_vars(Timeout)
    /\ WF_vars(JoinerAbortDiscard)
    /\ WF_vars(RetryOpen)

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety properties.                                                       *)
(***************************************************************************)

CommittedMesh(p, f) == f <= committedUpTo[p] /\ mode[p][f] # NoMode

JoinerCommitted(f) ==
    /\ joinerMode[f] # NoMode
    /\ joinerCommittedF

(***************************************************************************)
(* S1 -- Agreement (unchanged statement, larger space): any two peers that  *)
(* both committed a frame committed the SAME VALUE there. With deferred     *)
(* commits + the abort restore this now also checks the post-abort world:   *)
(* every peer (including survivors that reopened and were restored)         *)
(* commits the aborted attempt's frame FROZEN with the agreed value. A      *)
(* kept real-at-F commit across an abort -- the behavior the previous       *)
(* model documented -- would violate this the moment any other peer froze   *)
(* that frame.                                                              *)
(***************************************************************************)
AgreementMeshMesh ==
    \A p, q \in MeshPeers:
        \A f \in Frames:
            (CommittedMesh(p, f) /\ CommittedMesh(q, f)) => val[p][f] = val[q][f]

AgreementMeshJoiner ==
    \A p \in MeshPeers:
        \A f \in Frames:
            (CommittedMesh(p, f) /\ JoinerCommitted(f)) => val[p][f] = joinerVal[f]

Agreement ==
    /\ AgreementMeshMesh
    /\ AgreementMeshJoiner

NoConfirmedRewrite ==
    \A p \in MeshPeers:
        \A f \in Frames:
            (f <= committedUpTo[p]) => (mode[p][f] # NoMode /\ val[p][f] # NoVal)

(***************************************************************************)
(* L1 -- NoSplitBrainOnAbort, per attempt: no aborted attempt has the       *)
(* joiner committed real at its activation frame while some survivor        *)
(* committed it frozen.                                                     *)
(***************************************************************************)
SurvivorFrozenAt(f) ==
    \E s \in Survivors: CommittedMesh(s, f) /\ mode[s][f] = "frozen"

JoinerRealAt(f) == JoinerCommitted(f) /\ joinerMode[f] = "real"

NoSplitBrainOnAbort ==
    /\ (phase \in {"aborted", "paused2", "joined2", "aborted2"}) =>
         ~(JoinerRealAt(AttF(1)) /\ SurvivorFrozenAt(AttF(1)))
    /\ (phase = "aborted2") =>
         ~(JoinerRealAt(AttF(2)) /\ SurvivorFrozenAt(AttF(2)))

(***************************************************************************)
(* RealOnlyInCommittedAttempt (session-33, the abort-restore pin): a peer   *)
(* holds a REAL commit at an attempt's activation frame only if that        *)
(* attempt actually committed. Attempt 1 commits iff phase = "joined"       *)
(* (terminal); attempt 2 iff phase = "joined2". This is exactly what the    *)
(* implementation's JoinAborted restore guarantees -- and what the          *)
(* previous model's keep-the-real-commit behavior would violate.            *)
(***************************************************************************)
RealOnlyInCommittedAttempt ==
    \A p \in MeshPeers:
        /\ (mode[p][AttF(1)] = "real") => phase = "joined"
        /\ (mode[p][AttF(2)] = "real") => phase \in {"joined", "joined2"}

(***************************************************************************)
(* NoStaleReopen (session-33, the stale-directive guard pin): no survivor   *)
(* ever holds a pending attempt at or below its closed high-water -- i.e.   *)
(* a closed attempt's straggler directives can never re-enter. (Remove the  *)
(* AcceptableFresh high-water conjunct and TLC produces the wedge           *)
(* counterexample: a JoinAborted{F1}-closed survivor re-accepts a stale     *)
(* attempt-1 directive whose lifecycle no longer exists.)                   *)
(***************************************************************************)
NoStaleReopen ==
    \A s \in Survivors:
        pendingA[s] # 0 => pendingA[s] > closedHW[s]

SafetyInvariant ==
    /\ TypeInvariant
    /\ Agreement
    /\ NoConfirmedRewrite
    /\ NoSplitBrainOnAbort
    /\ RealOnlyInCommittedAttempt
    /\ NoStaleReopen

(***************************************************************************)
(* Liveness: the protocol eventually reaches a committed join (either       *)
(* attempt) or the terminal second abort -- including from the wedge        *)
(* shape, where a survivor whose JoinAborted{F1} was lost sits              *)
(* reopened-at-F1 during attempt 2 and only the implied close (leg 3)       *)
(* lets attempt 2 gather its ack.                                           *)
(***************************************************************************)
EventuallyResolved == <>(phase \in {"joined", "joined2", "aborted2"})

StateConstraint ==
    /\ \A p \in MeshPeers: committedUpTo[p] <= MaxFrame
    /\ L <= MaxFrame

THEOREM Spec => []SafetyInvariant

================================================================================
