----------------------- MODULE SyncHandshakeV1Trace -----------------------
(***************************************************************************)
(* Executable trace contract for the bounded SyncHandshakeV1 model.        *)
(*                                                                         *)
(* TRACE is a sequence whose first record fixes every modeled variable in  *)
(* an Init state. Generated later records name one base action and fix the  *)
(* complete post-action state; source NDJSON rows carry strict deltas that  *)
(* the wrapper expands. TLC must consume every row through a base action.   *)
(***************************************************************************)

EXTENDS SyncHandshakeV1

CONSTANT TRACE

VARIABLE traceIndex

traceVars == <<
    phase, localConfig, learnedConfig, learnedFrom, syncRemaining,
    acceptedTokens, nextToken, timeoutTicks, timeoutEventCount,
    incompatibleEventCount, reasonField, reasonOurs, reasonTheirs,
    network, traceIndex
>>

TraceStateMatches(row) ==
    /\ phase = row.phase
    /\ localConfig = row.localConfig
    /\ learnedConfig = row.learnedConfig
    /\ learnedFrom = row.learnedFrom
    /\ syncRemaining = row.syncRemaining
    /\ acceptedTokens = row.acceptedTokens
    /\ nextToken = row.nextToken
    /\ timeoutTicks = row.timeoutTicks
    /\ timeoutEventCount = row.timeoutEventCount
    /\ incompatibleEventCount = row.incompatibleEventCount
    /\ reasonField = row.reasonField
    /\ reasonOurs = row.reasonOurs
    /\ reasonTheirs = row.reasonTheirs
    /\ network = row.network

TraceUpdateMatches(row) ==
    /\ phase' = row.phase
    /\ localConfig' = row.localConfig
    /\ learnedConfig' = row.learnedConfig
    /\ learnedFrom' = row.learnedFrom
    /\ syncRemaining' = row.syncRemaining
    /\ acceptedTokens' = row.acceptedTokens
    /\ nextToken' = row.nextToken
    /\ timeoutTicks' = row.timeoutTicks
    /\ timeoutEventCount' = row.timeoutEventCount
    /\ incompatibleEventCount' = row.incompatibleEventCount
    /\ reasonField' = row.reasonField
    /\ reasonOurs' = row.reasonOurs
    /\ reasonTheirs' = row.reasonTheirs
    /\ network' = row.network

TraceAction(row) ==
    CASE row.action = "SendSyncRequest" -> SendSyncRequest(row.peer)
      [] row.action = "HandleSyncRequest" -> HandleSyncRequest(row.peer)
      [] row.action = "HandleSyncReply" -> HandleSyncReply(row.peer)
      [] row.action = "TickTimeout" -> TickTimeout(row.peer)
      [] row.action = "ReportSyncTimeout" -> ReportSyncTimeout(row.peer)
      [] OTHER -> FALSE

TraceInit ==
    /\ Len(TRACE) > 0
    /\ TRACE[1].action = "Init"
    /\ Init
    /\ TraceStateMatches(TRACE[1])
    /\ traceIndex = 1

TraceStep ==
    /\ traceIndex < Len(TRACE)
    /\ LET row == TRACE[traceIndex + 1] IN
       /\ TraceAction(row)
       /\ TraceUpdateMatches(row)
    /\ traceIndex' = traceIndex + 1

TraceDone ==
    /\ traceIndex = Len(TRACE)
    /\ UNCHANGED traceVars

TraceNext == TraceStep \/ TraceDone

TraceSpec ==
    /\ TraceInit
    /\ [][TraceNext]_traceVars
    /\ WF_traceVars(TraceStep)

TraceTypeInvariant == traceIndex \in 1..Len(TRACE)

EventuallyTraceConsumed == <> (traceIndex = Len(TRACE))

=============================================================================
