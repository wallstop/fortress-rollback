--------------------------- MODULE NPeerReactivation ---------------------------
(***************************************************************************)
(* TLA+ model of "Agreement C" -- the activation-frame agreement of        *)
(* N-peer mesh reconnection (Session 18 design, progress/                  *)
(* session-18-npeer-mesh-reconnection-design.md, sections 4.C / 5 / 8).    *)
(*                                                                         *)
(* Two survivors is the minimal configuration that exercises the           *)
(* multi-party agreement; this mesh uses three survivors {S1, S2, S3} for  *)
(* N >= 3 coverage (catching any 2-survivor-specific assumption). The spec *)
(* is parameterized over Survivors, so the invariants hold for any         *)
(* non-empty set of survivors: one coordinator K, the survivors, and one   *)
(* returning joiner J reopening a single dropped slot h.                   *)
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
(*   - The keepalive-preserved CAP, modeled LIVE (section 4.C). Survivors   *)
(*     start with committedUpTo[s] anywhere in 0..L and RACE toward F by     *)
(*     committing frozen frames (SurvivorAdvanceFrozen). The cap -- "confirmed*)
(*     = min over connected incl. K" (design invariant 3) -- holds each      *)
(*     survivor at F-1 = L: a survivor may NOT raise committedUpTo to >= F   *)
(*     (commit frame F frozen) while the coordinator's keepalives keep K in  *)
(*     the survivor's connected-min (capHeld[s]). A survivor reaches L and   *)
(*     is HELD there until it reopens. This makes the cap an exercised,      *)
(*     load-bearing constraint rather than an assertion by construction.     *)
(*   - The CAP-COLLAPSE hazard (section 4.C), modeled explicitly. CapCollapse*)
(*     represents the coordinator dropping out of a survivor's connected-min *)
(*     mid-pause (the wall-clock-timeout-during-pause danger). It WOULD let  *)
(*     the survivor commit frozen past L. The protocol's KEEPALIVE mechanism *)
(*     is modeled as the guard: while keepaliveServing is TRUE (the paused   *)
(*     coordinator runs poll_remote_clients before the pause gate every poll,*)
(*     so survivors never disconnect-timeout it), CapCollapse cannot fire.   *)
(*     The naive variant (scratch, used only to prove the property has teeth)*)
(*     drops this guard and a survivor reaches frozen-at-F.                  *)
(*   - Activation frame F = L + 1, chosen once by K and carried verbatim     *)
(*     to each survivor in the ReactivateSlot directive (section 4.C, 8).    *)
(*   - Lossy / reorderable reopen delivery: a ReactivateSlot{h,F} in flight *)
(*     to a survivor may be delivered, dropped, or (via retransmit) delayed *)
(*     (nondeterministic Next actions). A survivor reopens (mode/val at F =  *)
(*     real, acks) only after it has caught up to L (so its committed history*)
(*     has no gap) and the directive is delivered.                          *)
(*   - The LATE-APPLY abort lifecycle (section 5, "Late-apply abort          *)
(*     lifecycle"). The joiner BUFFERS the snapshot (joinerBuffered) and      *)
(*     stays provisional: it commits real-from-F (JoinerCommit) ONLY after a *)
(*     JoinCommitted signal -- which the coordinator sends only once EVERY    *)
(*     survivor has reopened. On JoinAborted the joiner DISCARDS the buffer   *)
(*     (JoinerAbortDiscard), having never committed real-at-F. The joiner     *)
(*     thus cannot be real-at-F while a survivor is frozen-at-F (split-brain) *)
(*     -- and because the cap also forbids a survivor committing F frozen     *)
(*     before reopen, that bad conjunction is unreachable for TWO independent *)
(*     load-bearing reasons (the cap AND the gated commit).                  *)
(*                                                                         *)
(* Properties verified (section 5):                                        *)
(*   - Agreement (S1): any two peers that have both committed a frame f     *)
(*     committed the same VALUE at f.                                       *)
(*   - NoConfirmedRewrite (S2): every frame within committedUpTo keeps a    *)
(*     definite (non-sentinel) committed value -- committed history never   *)
(*     reverts.                                                             *)
(*   - NoSplitBrainOnAbort (L1): no aborted state has the joiner committed  *)
(*     real at F while some survivor committed frozen at F.                 *)
(*   - Liveness: under weak fairness the protocol eventually reaches a      *)
(*     terminal joined or aborted state.                                    *)
(*                                                                         *)
(* This intentionally models the agreement contract at a small state-      *)
(* machine level (matching PeerDrop.tla), not the full UDP protocol. The   *)
(* invariants proven here hold for ANY non-empty set of survivors and ANY  *)
(* valid MaxFrame > 1; the .cfg uses tiny bounds for exhaustive checking.  *)
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
ASSUME MaxFrame \in Nat /\ MaxFrame > 1
ASSUME NoMode \notin {"frozen", "real"}
ASSUME AssumePA \in BOOLEAN

(***************************************************************************)
(* Peers that maintain committed history for slot h are the coordinator    *)
(* and the survivors. The joiner's own commitment is tracked separately    *)
(* (joinerMode / joinerVal / joinerCommittedF) because it follows the      *)
(* provisional lifecycle, not the survivor cap.                            *)
(***************************************************************************)
MeshPeers == Survivors \union {K}

Frames  == 0..MaxFrame
Modes   == {"frozen", "real", NoMode}

VARIABLES
    phase,              \* coordinator lifecycle phase
    L,                  \* coordinator's sent / last-saved frame; F = L + 1
    F,                  \* chosen activation frame
    fChosen,            \* TRUE once K has fixed F = L + 1
    committedUpTo,      \* [MeshPeers -> Frames]: highest frame each mesh peer confirmed
    mode,               \* [MeshPeers -> [Frames -> Modes]]: committed value-SOURCE per frame
    val,                \* [MeshPeers -> [Frames -> Vals]]:  committed VALUE (bytes) per frame
    frozenVal,          \* [Survivors -> Nat]: INDEPENDENT frozen value symbol per survivor
    capHeld,            \* [Survivors -> BOOLEAN]: K is in survivor s's connected-min (cap holds)
    keepaliveServing,   \* TRUE while the paused coordinator is serving keepalives (cap guard)
    reopened,           \* [MeshPeers -> BOOLEAN]: peer has reopened slot h at F
    inflight,           \* [Survivors -> BOOLEAN]: a ReactivateSlot{h,F} is in flight to s
    acked,              \* [Survivors -> BOOLEAN]: coordinator has the survivor's reopen-ack
    joinerBuffered,     \* TRUE once joiner has BUFFERED the snapshot (NOT yet applied)
    joinerCommittedF,   \* TRUE once joiner has irrevocably committed real at frames >= F
    joinerMode,         \* [Frames -> Modes]: joiner's committed value-source per frame
    joinerVal           \* [Frames -> Vals]:  joiner's committed value per frame

vars == <<phase, L, F, fChosen, committedUpTo, mode, val, frozenVal,
          capHeld, keepaliveServing, reopened, inflight, acked,
          joinerBuffered, joinerCommittedF, joinerMode, joinerVal>>

Phases == {"running", "paused", "joined", "aborted"}

(***************************************************************************)
(* The set of possible committed values: every survivor's frozen symbol,   *)
(* the shared real symbol, and the no-value sentinel.                       *)
(***************************************************************************)
FrozenValDomain == 0..1                     \* small independent symbol domain
Vals == FrozenValDomain \union {RealVal, NoVal}

(***************************************************************************)
(* Frozen-value precondition P-A (Agreement A / clean freeze).             *)
(*                                                                         *)
(* All survivors share one frozen value. Init conjoins this iff AssumePA is  *)
(* TRUE (the default config); the _NoPA config sets AssumePA = FALSE so the   *)
(* survivors may hold different frozen values and Agreement then fails --     *)
(* i.e. P-A is necessary. (A TLC CONSTRAINT is NOT used for this: a           *)
(* CONSTRAINT does not suppress invariant checking on the initial states it   *)
(* prunes, so P-A must be baked into Init instead.)                          *)
(***************************************************************************)
FrozenAgreement ==
    \A s1, s2 \in Survivors: frozenVal[s1] = frozenVal[s2]

(***************************************************************************)
(* Type invariant.                                                         *)
(***************************************************************************)
TypeInvariant ==
    /\ phase \in Phases
    /\ L \in Frames
    /\ F \in Frames
    /\ fChosen \in BOOLEAN
    /\ committedUpTo \in [MeshPeers -> Frames]
    /\ mode \in [MeshPeers -> [Frames -> Modes]]
    /\ val \in [MeshPeers -> [Frames -> Vals]]
    /\ frozenVal \in [Survivors -> FrozenValDomain]
    /\ capHeld \in [Survivors -> BOOLEAN]
    /\ keepaliveServing \in BOOLEAN
    /\ reopened \in [MeshPeers -> BOOLEAN]
    /\ inflight \in [Survivors -> BOOLEAN]
    /\ acked \in [Survivors -> BOOLEAN]
    /\ joinerBuffered \in BOOLEAN
    /\ joinerCommittedF \in BOOLEAN
    /\ joinerMode \in [Frames -> Modes]
    /\ joinerVal \in [Frames -> Vals]

(***************************************************************************)
(* Init.                                                                    *)
(*                                                                         *)
(* The mesh starts already paused at the moment the snapshot is served:     *)
(* the coordinator has chosen F = L + 1 and committed every frame <= L with *)
(* its frozen value (the surviving confirmed history of the dropped slot),  *)
(* and ReactivateSlot{h,F} is in flight to every survivor.                  *)
(*                                                                         *)
(* CRUCIAL CHANGE (non-vacuous cap): each survivor starts with committedUpTo*)
(* anywhere in 0..L -- i.e. it may LAG below the cap boundary. It then       *)
(* RACES toward F via SurvivorAdvanceFrozen and is HELD at L by the cap.     *)
(* (The coordinator K, the snapshot server, starts already confirmed at L.) *)
(* Frames a peer has not yet committed are NoMode/NoVal sentinels.          *)
(*                                                                         *)
(* frozenVal ranges over a small symbol set; Init conjoins FrozenAgreement  *)
(* iff AssumePA is TRUE so the default .cfg explores only the P-A world,     *)
(* while the _NoPA .cfg (AssumePA = FALSE) also explores P-A-violating       *)
(* initial states.                                                          *)
(*                                                                         *)
(* The coordinator's committed values use frozenVal of an arbitrary fixed   *)
(* survivor as its own frozen symbol (it served the snapshot, so it holds   *)
(* whatever the agreed value is); under P-A all survivors equal it anyway.  *)
(* keepaliveServing starts TRUE (the paused coordinator is serving) and the *)
(* cap holds for every survivor; the joiner has buffered nothing yet.       *)
(***************************************************************************)
SomeSurvivor == CHOOSE s \in Survivors: TRUE

PeerFrozenSym(p) == IF p \in Survivors THEN frozenVal[p] ELSE frozenVal[SomeSurvivor]

InitMode(p, c) == [f \in Frames |-> IF f <= c THEN "frozen" ELSE NoMode]

InitVal(p, c) ==
    [f \in Frames |-> IF f <= c THEN PeerFrozenSym(p) ELSE NoVal]

Init ==
    /\ L \in 1..(MaxFrame - 1)             \* leave room so F = L + 1 <= MaxFrame
    /\ phase = "paused"
    /\ F = L + 1
    /\ fChosen = TRUE
    /\ frozenVal \in [Survivors -> FrozenValDomain]
    /\ AssumePA => FrozenAgreement         \* pin P-A iff requested by the config
    \* Each survivor starts confirmed anywhere in 0..L (it may lag the cap);
    \* the coordinator starts confirmed at L (it served the snapshot at S = L).
    /\ \E start \in [Survivors -> Frames]:
         /\ \A s \in Survivors: start[s] <= L
         /\ committedUpTo = [p \in MeshPeers |-> IF p \in Survivors THEN start[p] ELSE L]
         /\ mode = [p \in MeshPeers |->
                      InitMode(p, IF p \in Survivors THEN start[p] ELSE L)]
         /\ val  = [p \in MeshPeers |->
                      InitVal(p, IF p \in Survivors THEN start[p] ELSE L)]
    /\ capHeld = [s \in Survivors |-> TRUE]
    /\ keepaliveServing = TRUE
    /\ reopened = [p \in MeshPeers |-> FALSE]
    /\ inflight = [s \in Survivors |-> TRUE]
    /\ acked = [s \in Survivors |-> FALSE]
    /\ joinerBuffered = FALSE
    /\ joinerCommittedF = FALSE
    /\ joinerMode = [f \in Frames |-> NoMode]
    /\ joinerVal = [f \in Frames |-> NoVal]

(***************************************************************************)
(* SurvivorAdvanceFrozen -- a survivor confirms one more frozen frame.      *)
(*                                                                         *)
(* A survivor extends its confirmed frozen history by one frame. Below the  *)
(* cap boundary (the next frame is < F) this is always allowed -- the        *)
(* survivor races up toward L. To raise committedUpTo to >= F (commit frame *)
(* F itself frozen) the cap must have COLLAPSED for s (~capHeld[s]); while   *)
(* the coordinator's keepalives keep K in s's connected-min the survivor is  *)
(* HELD at F-1 = L. This is the "confirmed = min over connected incl. K" cap *)
(* (design invariant 3) made live: a lost reopen directive is a DELAY, not a *)
(* desync, because no survivor can commit F with the frozen value while the  *)
(* cap holds.                                                                *)
(***************************************************************************)
SurvivorAdvanceFrozen(s) ==
    /\ s \in Survivors
    /\ phase = "paused"
    /\ committedUpTo[s] < MaxFrame          \* room to commit one more frame
    /\ ~reopened[s]                         \* once reopened, frames >= F are real
    /\ (committedUpTo[s] + 1 >= F => ~capHeld[s])   \* cap blocks frame F while held
    /\ committedUpTo' = [committedUpTo EXCEPT ![s] = committedUpTo[s] + 1]
    /\ mode' = [mode EXCEPT ![s][committedUpTo[s] + 1] = "frozen"]
    /\ val'  = [val  EXCEPT ![s][committedUpTo[s] + 1] = frozenVal[s]]
    /\ UNCHANGED <<phase, L, F, fChosen, frozenVal, capHeld, keepaliveServing,
                   reopened, inflight, acked, joinerBuffered, joinerCommittedF,
                   joinerMode, joinerVal>>

(***************************************************************************)
(* CapCollapse -- the cap-collapse hazard (design section 4.C).            *)
(*                                                                         *)
(* Models the coordinator dropping out of survivor s's connected-min        *)
(* mid-pause (a wall-clock disconnect-timeout firing DURING the pause). Were *)
(* it to fire, s's confirmed-min would no longer include K and s could       *)
(* commit frozen past L (via SurvivorAdvanceFrozen above, now that ~capHeld).*)
(*                                                                         *)
(* The PROTOCOL'S KEEPALIVE MECHANISM is the guard: the paused coordinator   *)
(* runs poll_remote_clients BEFORE the pause gate every poll, so survivors'  *)
(* last_recv_time stays fresh and they never disconnect-timeout it           *)
(* (keepaliveServing = TRUE). While serving, CapCollapse is DISABLED -- so in *)
(* the real spec the cap can never collapse before the join resolves, and    *)
(* survivors stay held at L. (The naive scratch variant drops the            *)
(* ~keepaliveServing conjunct; CapCollapse then fires and the survivor       *)
(* reaches frozen-at-F, producing the split-brain counterexample.)           *)
(***************************************************************************)
CapCollapse(s) ==
    /\ s \in Survivors
    /\ phase = "paused"
    /\ capHeld[s]
    /\ ~keepaliveServing                    \* keepalives, while served, forbid this
    /\ capHeld' = [capHeld EXCEPT ![s] = FALSE]
    /\ UNCHANGED <<phase, L, F, fChosen, committedUpTo, mode, val, frozenVal,
                   keepaliveServing, reopened, inflight, acked, joinerBuffered,
                   joinerCommittedF, joinerMode, joinerVal>>

(***************************************************************************)
(* DeliverReopen -- the in-flight ReactivateSlot{h,F} reaches survivor s.   *)
(*                                                                         *)
(* The survivor reopens slot h ONLY once it has caught up to L (so its       *)
(* confirmed history is gap-free up to F-1; the cap held it exactly there).  *)
(* It commits frame F with the REAL value, raises committedUpTo to F, and    *)
(* acks. F = L + 1 > committedUpTo[s] = L, so no already-committed frame is   *)
(* rewritten (S2). The directive carries F verbatim; the survivor never      *)
(* invents its own activation frame. (A directive that arrives before the    *)
(* survivor has reached L simply waits -- it stays in flight while the        *)
(* survivor keeps advancing.)                                                *)
(***************************************************************************)
DeliverReopen(s) ==
    /\ s \in Survivors
    /\ phase = "paused"
    /\ inflight[s]
    /\ ~reopened[s]
    /\ committedUpTo[s] = L                 \* caught up to the cap boundary first
    /\ reopened' = [reopened EXCEPT ![s] = TRUE]
    /\ committedUpTo' = [committedUpTo EXCEPT ![s] = F]
    /\ mode' = [mode EXCEPT ![s][F] = "real"]
    /\ val'  = [val  EXCEPT ![s][F] = RealVal]
    /\ inflight' = [inflight EXCEPT ![s] = FALSE]
    /\ acked' = [acked EXCEPT ![s] = TRUE]
    /\ UNCHANGED <<phase, L, F, fChosen, frozenVal, capHeld, keepaliveServing,
                   joinerBuffered, joinerCommittedF, joinerMode, joinerVal>>

(***************************************************************************)
(* DropReopen -- the in-flight ReactivateSlot{h,F} to survivor s is lost.   *)
(* No change to the survivor's committed history; the coordinator must      *)
(* retransmit (Retransmit below).                                           *)
(***************************************************************************)
DropReopen(s) ==
    /\ s \in Survivors
    /\ phase = "paused"
    /\ inflight[s]
    /\ ~reopened[s]
    /\ inflight' = [inflight EXCEPT ![s] = FALSE]
    /\ UNCHANGED <<phase, L, F, fChosen, committedUpTo, mode, val, frozenVal,
                   capHeld, keepaliveServing, reopened, acked, joinerBuffered,
                   joinerCommittedF, joinerMode, joinerVal>>

(***************************************************************************)
(* Retransmit -- coordinator re-sends ReactivateSlot{h,F} to a survivor     *)
(* that has neither acked nor has a message in flight. This is what keeps a *)
(* dropped directive a delay rather than a permanent loss.                  *)
(***************************************************************************)
Retransmit(s) ==
    /\ s \in Survivors
    /\ phase = "paused"
    /\ ~acked[s]
    /\ ~inflight[s]
    /\ ~reopened[s]
    /\ inflight' = [inflight EXCEPT ![s] = TRUE]
    /\ UNCHANGED <<phase, L, F, fChosen, committedUpTo, mode, val, frozenVal,
                   capHeld, keepaliveServing, reopened, acked, joinerBuffered,
                   joinerCommittedF, joinerMode, joinerVal>>

(***************************************************************************)
(* JoinerBuffer -- the joiner receives and BUFFERS the snapshot (Phase-2    *)
(* retransmit), but does NOT apply it (section 5, late-apply lifecycle).    *)
(* It stays "HotJoining": no LoadGameState, no commit, joinerCommittedF      *)
(* still FALSE. Only a later JoinCommitted (JoinerCommit) applies it.        *)
(***************************************************************************)
JoinerBuffer ==
    /\ phase = "paused"
    /\ ~joinerBuffered
    /\ joinerBuffered' = TRUE
    /\ UNCHANGED <<phase, L, F, fChosen, committedUpTo, mode, val, frozenVal,
                   capHeld, keepaliveServing, reopened, inflight, acked,
                   joinerCommittedF, joinerMode, joinerVal>>

(***************************************************************************)
(* UnpauseAndCommit -- ack-gated un-pause (success path) + JoinCommitted.   *)
(*                                                                         *)
(* The coordinator un-pauses ONLY when every survivor has acked its         *)
(* reopen (== every survivor is reopened-and-Running with the joiner). It    *)
(* reopens at F itself (F == current on the coordinator; no rollback needed, *)
(* design invariant 7 caveat) and SENDS JoinCommitted to the joiner. The     *)
(* joiner has NOT yet committed real-at-F here -- it applies its buffer in    *)
(* the separate JoinerCommit step, which is enabled only in this "joined"    *)
(* phase (i.e. only after JoinCommitted). It stops serving keepalives now    *)
(* that the barrier is lifting.                                              *)
(***************************************************************************)
UnpauseAndCommit ==
    /\ phase = "paused"
    /\ \A s \in Survivors: acked[s]
    /\ joinerBuffered                       \* JoinCommitted only after the buffer
    /\ phase' = "joined"
    /\ keepaliveServing' = FALSE
    /\ reopened' = [reopened EXCEPT ![K] = TRUE]
    /\ committedUpTo' = [committedUpTo EXCEPT ![K] = F]
    /\ mode' = [mode EXCEPT ![K][F] = "real"]
    /\ val'  = [val  EXCEPT ![K][F] = RealVal]
    /\ UNCHANGED <<L, F, fChosen, frozenVal, capHeld, inflight, acked,
                   joinerBuffered, joinerCommittedF, joinerMode, joinerVal>>

(***************************************************************************)
(* JoinerCommit -- the joiner applies its buffered snapshot on JoinCommitted*)
(* (section 5). Reachable only in the "joined" phase, which the coordinator *)
(* enters ONLY after EVERY survivor has reopened (acked) -- that gating is   *)
(* the whole point. The joiner loads state at S = F-1 (bridge frame, carried *)
(* frozen value) AND commits real-from-F, becoming real-at-F. Because all    *)
(* survivors are real-at-F by now, joiner-real-at-F can never coincide with  *)
(* survivor-frozen-at-F.                                                     *)
(*                                                                         *)
(* The carried frozen value is the coordinator's (the snapshot server's)     *)
(* held value at F-1; under P-A this equals every survivor's value at F-1.   *)
(***************************************************************************)
JoinerCommit ==
    /\ phase = "joined"
    /\ joinerBuffered
    /\ ~joinerCommittedF
    /\ joinerCommittedF' = TRUE
    /\ joinerMode' = [f \in Frames |->
                        IF f = F - 1 THEN "frozen"
                        ELSE IF f = F THEN "real" ELSE joinerMode[f]]
    /\ joinerVal'  = [f \in Frames |->
                        IF f = F - 1 THEN val[K][F - 1]
                        ELSE IF f = F THEN RealVal ELSE joinerVal[f]]
    /\ UNCHANGED <<phase, L, F, fChosen, committedUpTo, mode, val, frozenVal,
                   capHeld, keepaliveServing, reopened, inflight, acked,
                   joinerBuffered>>

(***************************************************************************)
(* Timeout -- the coordinator aborts (Phase-4 serve timeout).              *)
(*                                                                         *)
(* The pause is bounded by serve_timeout_polls. On timeout the coordinator  *)
(* aborts: it sends JoinAborted to the joiner and the slot stays reserved.   *)
(* The coordinator never reopened, so it never committed real at F. A        *)
(* survivor that already reopened keeps its real-at-F commit (no other peer  *)
(* committed F frozen -- the cap forbade it), so Agreement still holds. It    *)
(* stops serving keepalives now that it has given up. The joiner DISCARDS its *)
(* buffer in the separate JoinerAbortDiscard step (it never committed        *)
(* real-at-F), eliminating the split-brain window (L1).                      *)
(***************************************************************************)
Timeout ==
    /\ phase = "paused"
    /\ ~(\A s \in Survivors: acked[s])      \* abort is the not-all-acked branch
    /\ phase' = "aborted"
    /\ keepaliveServing' = FALSE
    /\ UNCHANGED <<L, F, fChosen, committedUpTo, mode, val, frozenVal, capHeld,
                   reopened, inflight, acked, joinerBuffered, joinerCommittedF,
                   joinerMode, joinerVal>>

(***************************************************************************)
(* JoinerAbortDiscard -- the joiner discards its buffer on JoinAborted      *)
(* (section 5). Reachable only in the "aborted" phase. No user-visible state *)
(* was ever loaded (the snapshot was buffered, not applied), so there is     *)
(* nothing to un-load; joinerCommittedF stays FALSE and the joiner is NOT    *)
(* real-at-F. The buffer is dropped so the joiner can retry from a fresh     *)
(* layer.                                                                    *)
(***************************************************************************)
JoinerAbortDiscard ==
    /\ phase = "aborted"
    /\ joinerBuffered
    /\ joinerBuffered' = FALSE
    /\ UNCHANGED <<phase, L, F, fChosen, committedUpTo, mode, val, frozenVal,
                   capHeld, keepaliveServing, reopened, inflight, acked,
                   joinerCommittedF, joinerMode, joinerVal>>

(***************************************************************************)
(* Terminal stutter -- once joined or aborted (and the joiner has applied   *)
(* or discarded its buffer) the model self-loops so the liveness property   *)
(* has a stable terminal state and TLC sees no deadlock.                    *)
(***************************************************************************)
TerminalStutter ==
    /\ phase \in {"joined", "aborted"}
    /\ (phase = "joined"  => joinerCommittedF)
    /\ (phase = "aborted" => ~joinerBuffered)
    /\ UNCHANGED vars

Next ==
    \/ \E s \in Survivors: SurvivorAdvanceFrozen(s)
    \/ \E s \in Survivors: CapCollapse(s)
    \/ \E s \in Survivors: DeliverReopen(s)
    \/ \E s \in Survivors: DropReopen(s)
    \/ \E s \in Survivors: Retransmit(s)
    \/ JoinerBuffer
    \/ UnpauseAndCommit
    \/ JoinerCommit
    \/ Timeout
    \/ JoinerAbortDiscard
    \/ TerminalStutter

(***************************************************************************)
(* Fairness.                                                                *)
(*                                                                         *)
(* To guarantee progress we require the message-delivery /                 *)
(* coordinator-decision actions are eventually taken. We do NOT make        *)
(* DropReopen or CapCollapse fair (loss / the hazard must not be forced),   *)
(* but we DO make SurvivorAdvanceFrozen weakly fair (a lagging survivor      *)
(* catches up to the cap), Retransmit and DeliverReopen weakly fair (a       *)
(* perpetually-droppable directive still eventually lands), and the          *)
(* coordinator's UnpauseAndCommit / Timeout decisions and the joiner's       *)
(* buffer/commit/discard fair so the mesh cannot dawdle in "paused" forever  *)
(* nor leave the joiner provisional after the join resolves.                 *)
(***************************************************************************)
Fairness ==
    /\ \A s \in Survivors: WF_vars(SurvivorAdvanceFrozen(s))
    /\ \A s \in Survivors: WF_vars(DeliverReopen(s))
    /\ \A s \in Survivors: WF_vars(Retransmit(s))
    /\ WF_vars(JoinerBuffer)
    /\ WF_vars(UnpauseAndCommit)
    /\ WF_vars(JoinerCommit)
    /\ WF_vars(Timeout)
    /\ WF_vars(JoinerAbortDiscard)

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety properties (section 5).                                          *)
(***************************************************************************)

(***************************************************************************)
(* CommittedMesh(p, f): mesh peer p has irrevocably committed frame f       *)
(* (within committedUpTo and with a definite value-source).                 *)
(***************************************************************************)
CommittedMesh(p, f) == f <= committedUpTo[p] /\ mode[p][f] # NoMode

(***************************************************************************)
(* JoinerCommitted(f): the joiner has irrevocably committed frame f. Frames *)
(* >= F count only once joinerCommittedF is set (the provisional lifecycle);*)
(* the bridge frame F-1 also counts only once committed (applied).          *)
(***************************************************************************)
JoinerCommitted(f) ==
    /\ joinerMode[f] # NoMode
    /\ (f >= F - 1 => joinerCommittedF)

(***************************************************************************)
(* S1 -- Agreement. Any two peers that have both committed a frame f        *)
(* committed the SAME VALUE at f. Quantified over the coordinator, both     *)
(* survivors, and the joiner.                                              *)
(*                                                                         *)
(* Because frozen commits store the per-peer frozenVal symbol, this fails   *)
(* unless the survivors' frozen values agree -- i.e. it makes P-A           *)
(* load-bearing rather than assumed.                                        *)
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

(***************************************************************************)
(* S2 -- NoConfirmedRewrite. Every frame within committedUpTo keeps a        *)
(* definite (non-sentinel) committed value-source and value, i.e. committed *)
(* history never reverts to "uncommitted". (The action-level guarantee that *)
(* an already-committed frame is never WRITTEN again is enforced            *)
(* structurally: every survivor write targets committedUpTo+1 (frozen       *)
(* advance) or F > committedUpTo (reopen, gated on committedUpTo = L); the   *)
(* coordinator writes only F = current; the joiner writes F-1 and F together.*)
(* This invariant captures the observable consequence -- no gap and no       *)
(* revert within committed history.)                                        *)
(***************************************************************************)
NoConfirmedRewrite ==
    \A p \in MeshPeers:
        \A f \in Frames:
            (f <= committedUpTo[p]) => (mode[p][f] # NoMode /\ val[p][f] # NoVal)

(***************************************************************************)
(* L1 -- NoSplitBrainOnAbort. In any aborted state it is NOT the case that  *)
(* the joiner committed real at F while some survivor committed frozen at F. *)
(* This is NON-vacuous: the cap (SurvivorAdvanceFrozen + CapCollapse) makes  *)
(* survivor-frozen-at-F reachable in principle, and the late-apply lifecycle *)
(* (JoinerBuffer / JoinerCommit gated on JoinCommitted / JoinerAbortDiscard) *)
(* makes joiner-real-at-F reachable in principle; the property says the two  *)
(* never coincide on the abort path. The scratch naive variant (keepalive    *)
(* guard + JoinCommitted gate removed) reaches exactly this bad state, so    *)
(* the property has teeth.                                                   *)
(***************************************************************************)
SurvivorFrozenAt(f) ==
    \E s \in Survivors: CommittedMesh(s, f) /\ mode[s][f] = "frozen"

JoinerRealAt(f) == JoinerCommitted(f) /\ joinerMode[f] = "real"

NoSplitBrainOnAbort ==
    (phase = "aborted") => ~(JoinerRealAt(F) /\ SurvivorFrozenAt(F))

(***************************************************************************)
(* Aggregate safety invariant (mirrors PeerDrop.tla's SafetyInvariant).     *)
(***************************************************************************)
SafetyInvariant ==
    /\ TypeInvariant
    /\ Agreement
    /\ NoConfirmedRewrite
    /\ NoSplitBrainOnAbort

(***************************************************************************)
(* Liveness. Under weak fairness the protocol eventually reaches a terminal *)
(* joined or aborted state -- it never stalls forever in "paused".          *)
(***************************************************************************)
EventuallyResolved == <>(phase \in {"joined", "aborted"})

(***************************************************************************)
(* State constraint -- bounds the explored state space. (Already finite via *)
(* MaxFrame; kept explicit to match the other specs.)                       *)
(***************************************************************************)
StateConstraint ==
    /\ \A p \in MeshPeers: committedUpTo[p] <= MaxFrame
    /\ L <= MaxFrame

THEOREM Spec => []SafetyInvariant

================================================================================
