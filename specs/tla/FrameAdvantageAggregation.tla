----------------------- MODULE FrameAdvantageAggregation -----------------------
(***************************************************************************)
(* TLA+ Specification for the cross-endpoint frame-advantage aggregation    *)
(* (P2PSession::max_frame_advantage), the N>=3 piece TimeSync.tla cannot      *)
(* express on its own.                                                      *)
(*                                                                         *)
(* WHAT THIS MODELS (and why it is the companion to TimeSync.tla)            *)
(*                                                                         *)
(* TimeSync.tla models ONE peer's rolling window and proves a single          *)
(* endpoint's average_frame_advantage() is bounded and deterministic. But      *)
(* the session-level wait recommendation is computed by FOLDING those          *)
(* per-endpoint averages across ALL remote endpoints                          *)
(* (src/sessions/p2p_session.rs::max_frame_advantage):                        *)
(*                                                                         *)
(*     let mut interval = i32::MIN;                                          *)
(*     for endpoint in self.player_reg.remotes.values() {                    *)
(*         for &handle in endpoint.handles().iter() {                        *)
(*             let status = self.local_connect_status[handle];               *)
(*             if !status.disconnected {                                     *)
(*                 interval = max(interval, endpoint.average_frame_advantage());*)
(*             }                                                             *)
(*         }                                                                 *)
(*     }                                                                     *)
(*     if interval == i32::MIN { interval = 0; }   // no connected remote     *)
(*     interval                                                              *)
(*                                                                         *)
(* This fold has exactly the structure the original audit flagged as          *)
(* unmodeled at N>=3 (the TimeSync.cfg pin: "a meaningful N>=3 version needs   *)
(* a cross-peer max_frame_advantage aggregation action"). A constant bump of   *)
(* TimeSync.tla cannot reach it (Session 27 proved that: the window spec has    *)
(* zero cross-peer interaction, so a 3rd peer only cubes the state space and    *)
(* adds no coverage). The aggregation is a genuinely separate concern, modeled  *)
(* here over an abstract per-endpoint average (TimeSync.tla already proves       *)
(* that average bounded and deterministic, so abstracting it is sound -- the     *)
(* same composition FreezeConvergence.tla uses to abstract InputQueue's ring).   *)
(*                                                                         *)
(* The fold's subtle, N>=3-only correctness properties are exactly the two       *)
(* arbitrated-NOTABUG findings F15 / completeness-critic #5:                     *)
(*   1. A multi-handle ("couch co-op") endpoint is folded ONCE PER HANDLE, but   *)
(*      max(x, x) = x, so the per-handle fold is IDEMPOTENT -- it must NOT        *)
(*      double-count avg into 2x. (max_frame_advantage_multi_handle_endpoint_    *)
(*      is_idempotent_not_additive)                                              *)
(*   2. A fully-disconnected endpoint (every owned handle disconnected) is        *)
(*      EXCLUDED -- it never contributes its average; if no endpoint is           *)
(*      connected the result falls back to 0. (max_frame_advantage_excludes_      *)
(*      disconnected_multi_handle_counts_connected_running)                       *)
(*                                                                         *)
(* The gate is faithfully the per-handle `!local_connect_status[h].disconnected`  *)
(* bit (NOT endpoint.is_running()); F15/critic #5 arbitrated that the two         *)
(* exclude the same set, so modeling the actual gate is both faithful and         *)
(* sufficient.                                                                    *)
(*                                                                         *)
(* Properties verified:                                                          *)
(*   - Safety: the faithful per-handle fold equals the order-independent max      *)
(*     over connected endpoints (FoldMatchesMaxSemantic -- the determinism /       *)
(*     order-independence core)                                                   *)
(*   - Safety: the per-handle fold equals the one-fold-per-endpoint fold           *)
(*     (MultiHandleIdempotent -- F15 / critic #5, exactly)                          *)
(*   - Safety: the result is always some CONNECTED endpoint's average, or 0         *)
(*     when none is connected (AggregateIsAContributorOrZero -- exclusion +          *)
(*     fallback in one statement)                                                    *)
(*   - Safety: no connected endpoint => result is 0 (FallbackZero -- the              *)
(*     i32::MIN sentinel never leaks)                                                *)
(*   - Safety: the result stays within the advantage bound (AggregateBounded)         *)
(*   - Safety: an emitted WaitRecommendation always carries a >= MIN_RECOMMENDATION    *)
(*     skip value (RecommendationPositive -- ties the fold to the public event)        *)
(***************************************************************************)

EXTENDS Integers, FiniteSets, TLC

CONSTANTS
    NUM_ENDPOINTS,          \* number of remote endpoints (>=3 models an N>=4 mesh, local + 3 remotes)
    MAX_ADVANTAGE,          \* bound on a per-endpoint average_frame_advantage()
    MULTI_HANDLE_COUNT,     \* handles owned by the one couch-co-op endpoint (>=2)
    MIN_RECOMMENDATION,     \* threshold to emit a WaitRecommendation (production: 3)
    NULL_REC                \* sentinel for "no recommendation emitted yet"

ASSUME NUM_ENDPOINTS \in Nat /\ NUM_ENDPOINTS >= 1
ASSUME MAX_ADVANTAGE \in Nat /\ MAX_ADVANTAGE > 0
ASSUME MULTI_HANDLE_COUNT \in Nat /\ MULTI_HANDLE_COUNT >= 2
ASSUME MIN_RECOMMENDATION \in Nat /\ MIN_RECOMMENDATION >= 1
\* The NULL sentinel must be outside the emittable recommendation range so it
\* is unambiguous (recommendations are always in [MIN_RECOMMENDATION, MAX_ADVANTAGE]).
ASSUME NULL_REC \notin MIN_RECOMMENDATION..MAX_ADVANTAGE

(***************************************************************************)
(* Type Definitions                                                        *)
(***************************************************************************)
Endpoints == 1..NUM_ENDPOINTS
AdvantageValue == (-MAX_ADVANTAGE)..MAX_ADVANTAGE

\* Endpoint 1 is the multi-handle ("couch co-op") endpoint; the rest own one
\* handle each. A fixed assignment (rather than a nondeterministic one) keeps
\* the model deterministic and small while still exercising the per-handle
\* fold more than once for endpoint 1 -- which is all the idempotence property
\* needs. Handle IDENTITIES are abstracted: only the per-endpoint handle COUNT
\* and the number of connected handles matter to max_frame_advantage's result.
HandleCount(e) == IF e = 1 THEN MULTI_HANDLE_COUNT ELSE 1

\* A value strictly below every real average, modeling the i32::MIN seed.
MinSentinel == -(MAX_ADVANTAGE + 1)

(***************************************************************************)
(* Variables                                                               *)
(***************************************************************************)
VARIABLES
    avg,                \* avg[e]            : endpoint e's average_frame_advantage()
    connectedCount,     \* connectedCount[e] : how many of e's handles are NOT disconnected
    lastRecommendation  \* the most recent emitted WaitRecommendation skip value (or NULL_REC)

vars == <<avg, connectedCount, lastRecommendation>>

(***************************************************************************)
(* Type Invariant                                                          *)
(***************************************************************************)
TypeInvariant ==
    /\ avg \in [Endpoints -> AdvantageValue]
    /\ connectedCount \in [Endpoints -> 0..MULTI_HANDLE_COUNT]
    /\ \A e \in Endpoints : connectedCount[e] <= HandleCount(e)
    /\ lastRecommendation \in {NULL_REC} \union (MIN_RECOMMENDATION..MAX_ADVANTAGE)

(***************************************************************************)
(* Initial State                                                           *)
(* Every endpoint starts connected (all handles up) with a zero average     *)
(* (matching the window's initial all-zero contents) and no recommendation   *)
(* emitted yet.                                                             *)
(***************************************************************************)
Init ==
    /\ avg = [e \in Endpoints |-> 0]
    /\ connectedCount = [e \in Endpoints |-> HandleCount(e)]
    /\ lastRecommendation = NULL_REC

(***************************************************************************)
(* Integer max.                                                            *)
(***************************************************************************)
MaxI(a, b) == IF a >= b THEN a ELSE b

(***************************************************************************)
(* THE FAITHFUL FOLD -- a faithful transcription of max_frame_advantage's     *)
(* nested loop. The outer recursion is the `for endpoint in remotes` loop;    *)
(* the inner recursion is the `for handle in endpoint.handles()` loop. Handle *)
(* j of endpoint e is connected iff j <= connectedCount[e] (handle identity    *)
(* is abstracted; only the count matters to the result). A connected handle    *)
(* folds avg[e] via max -- once per connected handle, exactly as production     *)
(* does.                                                                        *)
(*                                                                         *)
(* One production branch is deliberately ELIDED as unreachable: the            *)
(* `local_connect_status.get(handle)` None arm (Warning + `continue`,           *)
(* p2p_session.rs:8383-8391). local_connect_status always has num_players       *)
(* entries and every remote handle is in range, so that arm never fires for a   *)
(* validly-constructed session; the model treats every handle as present.       *)
(***************************************************************************)
RECURSIVE FoldHandlesOf(_, _, _)
FoldHandlesOf(e, k, acc) ==
    IF k = 0 THEN acc
    ELSE LET handleConnected == k <= connectedCount[e]
             acc2 == IF handleConnected THEN MaxI(acc, avg[e]) ELSE acc
         IN FoldHandlesOf(e, k - 1, acc2)

RECURSIVE FoldEndpoints(_, _)
FoldEndpoints(S, acc) ==
    IF S = {} THEN acc
    ELSE LET e == CHOOSE x \in S : TRUE
         IN FoldEndpoints(S \ {e}, FoldHandlesOf(e, HandleCount(e), acc))

\* The production result: fold everything from the i32::MIN seed, then map the
\* untouched-seed case to 0 ("if interval == i32::MIN { interval = 0 }").
RawAggregate == FoldEndpoints(Endpoints, MinSentinel)
Aggregate == IF RawAggregate = MinSentinel THEN 0 ELSE RawAggregate

(***************************************************************************)
(* REFERENCE SEMANTICS the fold is checked against.                          *)
(*                                                                         *)
(* An endpoint CONTRIBUTES iff at least one of its handles is connected --    *)
(* the per-handle `!disconnected` gate, lifted to the endpoint.              *)
(***************************************************************************)
ConnectedEndpoints == { e \in Endpoints : connectedCount[e] >= 1 }

\* Order-independent max over a set of endpoints' averages (the meaning the
\* commutative/associative/idempotent max fold should compute).
RECURSIVE MaxOverSet(_)
MaxOverSet(S) ==
    IF S = {} THEN MinSentinel
    ELSE LET e == CHOOSE x \in S : TRUE
         IN MaxI(avg[e], MaxOverSet(S \ {e}))

MaxSemanticRaw == MaxOverSet(ConnectedEndpoints)
MaxSemantic == IF MaxSemanticRaw = MinSentinel THEN 0 ELSE MaxSemanticRaw

\* The same fold but visiting each endpoint exactly ONCE (one notional handle),
\* gated on connectedCount[e] >= 1. This is what the per-handle fold MUST equal
\* for a multi-handle endpoint not to double-count -- F15 / critic #5, verbatim.
RECURSIVE FoldEndpointsSingle(_, _)
FoldEndpointsSingle(S, acc) ==
    IF S = {} THEN acc
    ELSE LET e == CHOOSE x \in S : TRUE
             acc2 == IF connectedCount[e] >= 1 THEN MaxI(acc, avg[e]) ELSE acc
         IN FoldEndpointsSingle(S \ {e}, acc2)
AggregateSingleRaw == FoldEndpointsSingle(Endpoints, MinSentinel)
AggregateSingle == IF AggregateSingleRaw = MinSentinel THEN 0 ELSE AggregateSingleRaw

(***************************************************************************)
(* Actions                                                                 *)
(***************************************************************************)

\* An endpoint's rolling window produced a new average (TimeSync feeds this).
SetAverage(e, v) ==
    /\ v \in AdvantageValue
    /\ avg' = [avg EXCEPT ![e] = v]
    /\ UNCHANGED <<connectedCount, lastRecommendation>>

\* A handle of endpoint e connects or disconnects in local_connect_status.
\* Lowering the count models a graceful drop / timeout; raising it models a
\* hot-join reactivation (disconnected -> connected). c ranges over 0..handles,
\* so a multi-handle endpoint can be partially connected -- more general than
\* production's per-endpoint-atomic disconnect, which is the stronger test of
\* the per-handle gate.
SetConnectedCount(e, c) ==
    /\ c \in 0..HandleCount(e)
    /\ connectedCount' = [connectedCount EXCEPT ![e] = c]
    /\ UNCHANGED <<avg, lastRecommendation>>

\* check_wait_recommendation: emit a WaitRecommendation only when the aggregate
\* frame advantage reaches the threshold. skip_frames is exactly the aggregate.
\* The production rate-limit gate (current_frame > next_recommended_sleep,
\* p2p_session.rs:8408) is omitted: it only suppresses WHETHER an event fires on
\* a given frame, never the skip VALUE, so it cannot affect RecommendationPositive.
RecommendWait ==
    /\ Aggregate >= MIN_RECOMMENDATION
    /\ lastRecommendation' = Aggregate
    /\ UNCHANGED <<avg, connectedCount>>

Next ==
    \/ \E e \in Endpoints, v \in AdvantageValue : SetAverage(e, v)
    \/ \E e \in Endpoints, c \in 0..MULTI_HANDLE_COUNT : SetConnectedCount(e, c)
    \/ RecommendWait

Spec == Init /\ [][Next]_vars

(***************************************************************************)
(* Safety Properties                                                       *)
(***************************************************************************)

\* The order-dependent-LOOKING per-handle/per-endpoint fold computes the
\* order-INDEPENDENT max over connected endpoints. This is simultaneously the
\* fold's correctness and its determinism: the result does not depend on the
\* iteration order of remotes.values() or endpoint.handles().
FoldMatchesMaxSemantic == Aggregate = MaxSemantic

\* F15 / completeness-critic #5, exactly: folding a multi-handle endpoint once
\* per handle yields the SAME result as folding it once per endpoint. An
\* additive (double-counting) fold would break this for any endpoint with >= 2
\* connected handles.
MultiHandleIdempotent == Aggregate = AggregateSingle

\* The result is always the average of SOME connected endpoint, or 0 if none is
\* connected. This pins both halves: a disconnected endpoint's average is never
\* the result (exclusion), and a phantom/sentinel value never is either.
AggregateIsAContributorOrZero ==
    IF ConnectedEndpoints = {}
        THEN Aggregate = 0
        ELSE \E e \in ConnectedEndpoints : Aggregate = avg[e]

\* The i32::MIN -> 0 fallback: when every endpoint is fully disconnected, the
\* recommendation aggregate is 0 (never a leaked sentinel, never stale).
FallbackZero == ConnectedEndpoints = {} => Aggregate = 0

\* The aggregate is bounded by the inputs (the 0 fallback is in range too).
AggregateBounded == Aggregate \in (-MAX_ADVANTAGE)..MAX_ADVANTAGE

\* Any recommendation actually emitted carried a skip value at or above the
\* threshold (so it was a genuine "you are ahead, slow down" signal, never a
\* spurious 0/negative emitted for an in-sync or disconnected mesh).
RecommendationPositive ==
    lastRecommendation # NULL_REC => lastRecommendation >= MIN_RECOMMENDATION

SafetyInvariant ==
    /\ TypeInvariant
    /\ FoldMatchesMaxSemantic
    /\ MultiHandleIdempotent
    /\ AggregateIsAContributorOrZero
    /\ FallbackZero
    /\ AggregateBounded
    /\ RecommendationPositive

(***************************************************************************)
(* Theorems                                                                *)
(***************************************************************************)
THEOREM Spec => []SafetyInvariant

================================================================================
