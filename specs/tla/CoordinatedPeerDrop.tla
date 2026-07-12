------------------------ MODULE CoordinatedPeerDrop ------------------------
(***************************************************************************)
(* Coordinated graceful-drop barrier for protocol v1.                      *)
(*                                                                         *)
(* This is a bounded safety model with a fair-delivery companion. It models*)
(* two serialized operation IDs and membership eras, explicit report      *)
(* payloads, value-bearing backfill, lossy/duplicate/reordered messages,    *)
(* independent local commits, stale-generation rejection, and fail-closed  *)
(* outcomes. One target input stream is modeled; endpoint/multi-handle      *)
(* composition remains a Rust-level obligation.                            *)
(***************************************************************************)

EXTENDS Naturals, FiniteSets, Sequences, TLC

CONSTANTS
    PEERS,
    COORD,
    MAX_FRAME,
    INPUTS,
    MAX_BACKFILL,
    MAX_GENERATION,
    MAX_NETWORK,
    MAX_TICKS,
    FIX_MODE,
    DELIVERY_MODE,
    INIT_MODE,
    NO_OP,
    NO_FRAME,
    NO_VALUE

ASSUME Cardinality(PEERS) = 2
ASSUME COORD \in PEERS
ASSUME MAX_FRAME = 1
ASSUME INPUTS # {}
ASSUME MAX_BACKFILL \in Nat /\ MAX_BACKFILL > 0
ASSUME MAX_GENERATION >= 1
ASSUME MAX_NETWORK \in Nat /\ MAX_NETWORK >= 2
ASSUME MAX_TICKS \in Nat /\ MAX_TICKS > 0
ASSUME FIX_MODE \in {"Barrier", "ImmediateMin"}
ASSUME DELIVERY_MODE \in {"ArbitraryLoss", "FairDelivery"}
ASSUME INIT_MODE \in {"All", "ReceiptWitness", "Mutation"}
ASSUME NO_OP \notin 1..2
ASSUME NO_FRAME \notin 0..MAX_FRAME
ASSUME NO_VALUE \notin INPUTS

Frames == 0..MAX_FRAME
Ops == 1..2
Generations == 0..MAX_GENERATION
FrameOrNone == Frames \union {NO_FRAME}
OpOrNone == Ops \union {NO_OP}
ValueOrNone == INPUTS \union {NO_VALUE}
PeerPhases == {"Running", "Prepared", "Ready", "Committed", "Failed"}
FailReasons == {"None", "Timeout", "MissingHistory", "OverBudget",
                 "Conflict", "ParticipantLoss", "GenerationChanged"}
MessageKinds == {"Prepare", "Report", "Cut", "Backfill", "Ready",
                 "Commit", "Fail"}

Other(p) == CHOOSE q \in PEERS: q # p
OperationEra(op) == op - 1
OpGeneration(op) == OperationEra(op)

Message == [kind: MessageKinds,
            op: Ops,
            era: Generations,
            from: PEERS,
            to: PEERS,
            generation: Generations,
            exposed: FrameOrNone,
            receipt: FrameOrNone,
            retained: FrameOrNone,
            cut: FrameOrNone,
            frame: FrameOrNone,
            value: ValueOrNone]

EmptyMessage(kind, op, from, to, generation) ==
    [kind |-> kind, op |-> op, era |-> OperationEra(op),
     from |-> from, to |-> to,
     generation |-> generation, exposed |-> NO_FRAME,
     receipt |-> NO_FRAME, retained |-> NO_FRAME, cut |-> NO_FRAME,
     frame |-> NO_FRAME, value |-> NO_VALUE]

PrepareMessage(op, to) ==
    EmptyMessage("Prepare", op, COORD, to, OpGeneration(op))

ReportMessage(op, from, e, r, retained) ==
    [EmptyMessage("Report", op, from, COORD, OpGeneration(op)) EXCEPT
        !.exposed = e, !.receipt = r, !.retained = retained]

CutMessage(op, to, c) ==
    [EmptyMessage("Cut", op, COORD, to, OpGeneration(op)) EXCEPT !.cut = c]

BackfillMessage(op, from, to, c, f, v) ==
    [EmptyMessage("Backfill", op, from, to, OpGeneration(op)) EXCEPT
        !.cut = c, !.frame = f, !.value = v]

ReadyMessage(op, from, c) ==
    [EmptyMessage("Ready", op, from, COORD, OpGeneration(op)) EXCEPT !.cut = c]

CommitMessage(op, from, to, c) ==
    [EmptyMessage("Commit", op, from, to, OpGeneration(op)) EXCEPT !.cut = c]

FailMessage(op, from, to) ==
    EmptyMessage("Fail", op, from, to, OpGeneration(op))

VARIABLES
    stream,
    baseReceipt,
    baseExposed,
    baseRetained,
    phase,
    activeOp,
    nextOp,
    completedOps,
    targetGeneration,
    lockedOp,
    exposed,
    receipt,
    retainedStart,
    history,
    heldAt,
    reportedExposed,
    reportedReceipt,
    reportedRetained,
    reportSeen,
    cut,
    cutChosen,
    knownCut,
    backfillCount,
    readySeen,
    certified,
    disconnected,
    frozenAt,
    commitCut,
    committedOp,
    committedGeneration,
    opReportedExposed,
    opCommitCut,
    eventCount,
    failedClosed,
    failReason,
    timeoutTicks,
    duplicateHandled,
    staleIgnored,
    conflictObserved,
    network,
    duplicateBudget

InNetwork(message) == message \in network

CanEnqueue(message) == Cardinality(network) < MAX_NETWORK /\ ~InNetwork(message)

MaxOf(S) == CHOOSE n \in S: \A m \in S: n >= m
MinOf(S) == CHOOSE n \in S: \A m \in S: n <= m
CurrentMembershipEra == MinOf({targetGeneration[p]: p \in PEERS})

ChosenCut ==
    IF FIX_MODE = "Barrier"
    THEN MaxOf({reportedExposed[p]: p \in PEERS} \union
               {reportedReceipt[p]: p \in PEERS})
    ELSE MinOf({reportedReceipt[p]: p \in PEERS})

InitialAvailable(start, last) == {f \in Frames: start <= f /\ f <= last}

HistoryFor(start, last, streamValue) ==
    [f \in Frames |-> IF f \in InitialAvailable(start, last)
                       THEN streamValue[f] ELSE NO_VALUE]

NeededFrames(p) ==
    IF knownCut[p] = NO_FRAME
    THEN {}
    ELSE {knownCut[p]} \union ((reportedReceipt[p] + 1)..knownCut[p])

MissingNeeded(p) == {f \in NeededFrames(p): history[p][f] = NO_VALUE}

AllNeededCanonical(p) ==
    knownCut[p] # NO_FRAME /\
    \A f \in NeededFrames(p): history[p][f] = stream[f]

vars == <<stream, baseReceipt, baseExposed, baseRetained, phase, activeOp,
          nextOp, completedOps, targetGeneration, lockedOp, exposed, receipt,
          retainedStart, history, heldAt, reportedExposed, reportedReceipt,
          reportedRetained, reportSeen, cut, cutChosen, knownCut,
          backfillCount, readySeen, certified, disconnected, frozenAt,
          commitCut, committedOp, committedGeneration, opReportedExposed,
          opCommitCut, eventCount, failedClosed, failReason, timeoutTicks,
          duplicateHandled, staleIgnored, conflictObserved, network,
          duplicateBudget>>

Init ==
    /\ stream \in [Frames -> INPUTS]
    /\ IF INIT_MODE = "ReceiptWitness"
       THEN /\ baseReceipt = [p \in PEERS |-> IF p = COORD THEN 0 ELSE 1]
            /\ baseExposed = [p \in PEERS |-> 0]
            /\ baseRetained = [p \in PEERS |-> 0]
       ELSE IF INIT_MODE = "Mutation"
            THEN /\ baseReceipt = [p \in PEERS |-> IF p = COORD THEN 0 ELSE 1]
                 /\ baseExposed = baseReceipt
                 /\ baseRetained = [p \in PEERS |-> 0]
       ELSE /\ baseReceipt \in [PEERS -> Frames]
            /\ baseExposed \in [PEERS -> Frames]
            /\ baseRetained \in [PEERS -> Frames]
            /\ \A p \in PEERS: baseExposed[p] <= baseReceipt[p]
            /\ \A p \in PEERS: baseRetained[p] <= baseReceipt[p]
    /\ phase = [p \in PEERS |-> "Running"]
    /\ activeOp = NO_OP
    /\ nextOp = 1
    /\ completedOps = {}
    /\ targetGeneration = [p \in PEERS |-> 0]
    /\ lockedOp = [p \in PEERS |-> NO_OP]
    /\ exposed = baseExposed
    /\ receipt = baseReceipt
    /\ retainedStart = baseRetained
    /\ history = [p \in PEERS |->
          HistoryFor(baseRetained[p], baseReceipt[p], stream)]
    /\ heldAt = [p \in PEERS |-> NO_FRAME]
    /\ reportedExposed = [p \in PEERS |-> NO_FRAME]
    /\ reportedReceipt = [p \in PEERS |-> NO_FRAME]
    /\ reportedRetained = [p \in PEERS |-> NO_FRAME]
    /\ reportSeen = {}
    /\ cut = NO_FRAME
    /\ cutChosen = FALSE
    /\ knownCut = [p \in PEERS |-> NO_FRAME]
    /\ backfillCount = [p \in PEERS |-> 0]
    /\ readySeen = {}
    /\ certified = FALSE
    /\ disconnected = [p \in PEERS |-> FALSE]
    /\ frozenAt = [p \in PEERS |-> NO_FRAME]
    /\ commitCut = [p \in PEERS |-> NO_FRAME]
    /\ committedOp = [p \in PEERS |-> NO_OP]
    /\ committedGeneration = [p \in PEERS |-> NO_FRAME]
    /\ opReportedExposed = [op \in Ops |-> [p \in PEERS |-> NO_FRAME]]
    /\ opCommitCut = [op \in Ops |-> [p \in PEERS |-> NO_FRAME]]
    /\ eventCount = [p \in PEERS |-> 0]
    /\ failedClosed = [p \in PEERS |-> FALSE]
    /\ failReason = [p \in PEERS |-> "None"]
    /\ timeoutTicks = [p \in PEERS |-> 0]
    /\ duplicateHandled = [p \in PEERS |-> FALSE]
    /\ staleIgnored = [p \in PEERS |-> FALSE]
    /\ conflictObserved = FALSE
    /\ network = {}
    /\ duplicateBudget = [op \in Ops |-> TRUE]

Advance(p) ==
    /\ phase[p] = "Running"
    /\ activeOp = NO_OP
    \* Keep the fair receipt witness asymmetric: the higher retained receipt,
    \* rather than a pre-prepare exposure step, must be what raises the cut.
    /\ INIT_MODE # "ReceiptWitness"
    /\ exposed[p] < receipt[p]
    /\ exposed' = [exposed EXCEPT ![p] = @ + 1]
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   receipt, retainedStart, history, heldAt, reportedExposed,
                   reportedReceipt, reportedRetained, reportSeen, cut,
                   cutChosen, knownCut, backfillCount, readySeen, certified,
                   disconnected, frozenAt, commitCut, committedOp,
                   committedGeneration, opReportedExposed, opCommitCut,
                   eventCount, failedClosed, failReason, timeoutTicks,
                   duplicateHandled, staleIgnored, conflictObserved, network,
                   duplicateBudget>>

Start ==
    /\ activeOp = NO_OP
    /\ nextOp \in Ops
    /\ \A p \in PEERS: phase[p] = "Running"
    /\ \A p \in PEERS: targetGeneration[p] = OpGeneration(nextOp)
    /\ activeOp' = nextOp
    /\ lockedOp' = [p \in PEERS |-> NO_OP]
    /\ heldAt' = [p \in PEERS |-> NO_FRAME]
    /\ reportedExposed' = [p \in PEERS |-> NO_FRAME]
    /\ reportedReceipt' = [p \in PEERS |-> NO_FRAME]
    /\ reportedRetained' = [p \in PEERS |-> NO_FRAME]
    /\ reportSeen' = {}
    /\ cut' = NO_FRAME
    /\ cutChosen' = FALSE
    /\ knownCut' = [p \in PEERS |-> NO_FRAME]
    /\ backfillCount' = [p \in PEERS |-> 0]
    /\ readySeen' = {}
    /\ certified' = FALSE
    /\ timeoutTicks' = [p \in PEERS |-> 0]
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   nextOp, completedOps, targetGeneration, exposed, receipt,
                   retainedStart, history, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   duplicateHandled, staleIgnored, conflictObserved, network,
                   duplicateBudget>>

SendPrepare(p) ==
    /\ activeOp \in Ops
    /\ phase[p] = "Running"
    /\ lockedOp[p] # activeOp
    /\ CanEnqueue(PrepareMessage(activeOp, p))
    /\ network' = network \union {PrepareMessage(activeOp, p)}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, certified, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, duplicateBudget>>

DeliverPrepare(msg) ==
    /\ msg \in network
    /\ msg.kind = "Prepare"
       /\ msg.op = activeOp
       /\ msg.era = CurrentMembershipEra
       /\ msg.generation = targetGeneration[msg.to]
       /\ phase[msg.to] = "Running"
       /\ phase' = [phase EXCEPT ![msg.to] = "Prepared"]
       /\ lockedOp' = [lockedOp EXCEPT ![msg.to] = msg.op]
       /\ heldAt' = [heldAt EXCEPT ![msg.to] = exposed[msg.to]]
       /\ reportedExposed' = [reportedExposed EXCEPT ![msg.to] = exposed[msg.to]]
       /\ reportedReceipt' = [reportedReceipt EXCEPT ![msg.to] = receipt[msg.to]]
       /\ reportedRetained' = [reportedRetained EXCEPT ![msg.to] = retainedStart[msg.to]]
       /\ opReportedExposed' = [opReportedExposed EXCEPT
              ![msg.op][msg.to] = exposed[msg.to]]
    /\ network' = network \ {msg}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, activeOp,
                   nextOp, completedOps, targetGeneration, exposed, receipt,
                   retainedStart, history, reportSeen, cut, cutChosen,
                   knownCut, backfillCount, readySeen, certified, disconnected,
                   frozenAt, commitCut, committedOp, committedGeneration,
                   opCommitCut, eventCount, failedClosed, failReason,
                   timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, duplicateBudget>>

SendReport(p) ==
    /\ activeOp \in Ops
    /\ phase[p] = "Prepared"
    /\ p \notin reportSeen
    /\ LET msg == ReportMessage(activeOp, p, reportedExposed[p],
                                reportedReceipt[p], reportedRetained[p]) IN
       /\ CanEnqueue(msg)
       /\ network' = network \union {msg}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, certified, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, duplicateBudget>>

DeliverReport(msg) ==
    /\ msg \in network
    /\ msg.kind = "Report"
       /\ msg.op = activeOp
       /\ msg.generation = OpGeneration(activeOp)
       /\ lockedOp[msg.from] = msg.op
       /\ msg.exposed = reportedExposed[msg.from]
       /\ msg.receipt = reportedReceipt[msg.from]
       /\ msg.retained = reportedRetained[msg.from]
       /\ reportSeen' = reportSeen \union {msg.from}
    /\ network' = network \ {msg}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained, cut,
                   cutChosen, knownCut, backfillCount, readySeen, certified,
                   disconnected, frozenAt, commitCut, committedOp,
                   committedGeneration, opReportedExposed, opCommitCut,
                   eventCount, failedClosed, failReason, timeoutTicks,
                   duplicateHandled, staleIgnored, conflictObserved,
                   duplicateBudget>>

ChooseCut ==
    /\ activeOp \in Ops
    /\ reportSeen = PEERS
    /\ ~cutChosen
    /\ cut' = ChosenCut
    /\ cutChosen' = TRUE
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, knownCut, backfillCount, readySeen, certified,
                   disconnected, frozenAt, commitCut, committedOp,
                   committedGeneration, opReportedExposed, opCommitCut,
                   eventCount, failedClosed, failReason, timeoutTicks,
                   duplicateHandled, staleIgnored, conflictObserved, network,
                   duplicateBudget>>

SendCut(p) ==
    /\ activeOp \in Ops
    /\ cutChosen
    /\ phase[p] = "Prepared"
    /\ knownCut[p] # cut
    /\ CanEnqueue(CutMessage(activeOp, p, cut))
    /\ network' = network \union {CutMessage(activeOp, p, cut)}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, certified, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, duplicateBudget>>

DeliverCut(msg) ==
    /\ msg \in network
    /\ msg.kind = "Cut"
       /\ msg.op = activeOp
       /\ msg.generation = targetGeneration[msg.to]
       /\ msg.cut = cut
       /\ lockedOp[msg.to] = msg.op
       /\ phase[msg.to] = "Prepared"
       /\ knownCut' = [knownCut EXCEPT ![msg.to] = msg.cut]
    /\ network' = network \ {msg}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, backfillCount, readySeen,
                   certified, disconnected, frozenAt, commitCut, committedOp,
                   committedGeneration, opReportedExposed, opCommitCut,
                   eventCount, failedClosed, failReason, timeoutTicks,
                   duplicateHandled, staleIgnored, conflictObserved,
                   duplicateBudget>>

SendBackfill(src, dst, f) ==
    /\ activeOp \in Ops
    /\ src # dst
    /\ phase[dst] = "Prepared"
    /\ knownCut[dst] # NO_FRAME
    /\ f \in MissingNeeded(dst)
    /\ history[src][f] # NO_VALUE
    /\ LET msg == BackfillMessage(activeOp, src, dst, knownCut[dst], f,
                                  history[src][f]) IN
       /\ CanEnqueue(msg)
       /\ network' = network \union {msg}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, certified, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, duplicateBudget>>

DeliverGoodBackfill(msg) ==
    /\ msg \in network
    /\ msg.kind = "Backfill"
       /\ msg.op = activeOp
       /\ msg.generation = targetGeneration[msg.to]
       /\ msg.cut = knownCut[msg.to]
       /\ msg.frame \in NeededFrames(msg.to)
       /\ msg.value = stream[msg.frame]
       /\ history[msg.to][msg.frame] = NO_VALUE
       /\ backfillCount[msg.to] < MAX_BACKFILL
       /\ history' = [history EXCEPT ![msg.to][msg.frame] = msg.value]
       /\ backfillCount' = [backfillCount EXCEPT ![msg.to] = @ + 1]
    \* Retain one equivalent copy so the explicit duplicate handler checks
    \* idempotence after the first value-bearing delivery.
    /\ UNCHANGED network
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, heldAt, reportedExposed,
                   reportedReceipt, reportedRetained, reportSeen, cut,
                   cutChosen, knownCut, readySeen, certified, disconnected,
                   frozenAt, commitCut, committedOp, committedGeneration,
                   opReportedExposed, opCommitCut, eventCount, failedClosed,
                   failReason, timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, duplicateBudget>>

DeliverDuplicateBackfill(msg) ==
    /\ msg \in network
    /\ msg.kind = "Backfill"
    /\ msg.op = activeOp
       /\ duplicateBudget[msg.op]
       /\ msg.generation = targetGeneration[msg.to]
       /\ msg.cut = knownCut[msg.to]
       /\ msg.frame \in NeededFrames(msg.to)
       /\ history[msg.to][msg.frame] = msg.value
       /\ duplicateHandled' = [duplicateHandled EXCEPT ![msg.to] = TRUE]
       /\ duplicateBudget' = [duplicateBudget EXCEPT ![msg.op] = FALSE]
    /\ network' = network \ {msg}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, certified, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   timeoutTicks, staleIgnored, conflictObserved>>

BeginFailure(p, reason) ==
    /\ activeOp \in Ops
    /\ phase[p] \in {"Running", "Prepared", "Ready"}
    /\ reason \in FailReasons \ {"None"}
    /\ phase' = [phase EXCEPT ![p] = "Failed"]
    /\ failedClosed' = [failedClosed EXCEPT ![p] = TRUE]
    /\ failReason' = [failReason EXCEPT ![p] = reason]
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, activeOp,
                   nextOp, completedOps, targetGeneration, lockedOp, exposed,
                   receipt, retainedStart, history, heldAt, reportedExposed,
                   reportedReceipt, reportedRetained, reportSeen, cut,
                   cutChosen, knownCut, backfillCount, readySeen, certified,
                   disconnected, frozenAt, commitCut, committedOp,
                   committedGeneration, opReportedExposed, opCommitCut,
                   eventCount, timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, network, duplicateBudget>>

DeliverConflictingBackfill(msg) ==
    /\ msg \in network
    /\ msg.kind = "Backfill"
       /\ msg.op = activeOp
       /\ msg.generation = targetGeneration[msg.to]
       /\ msg.frame \in NeededFrames(msg.to)
       /\ msg.value # stream[msg.frame]
       /\ phase[msg.to] \in {"Prepared", "Ready"}
       /\ phase' = [phase EXCEPT ![msg.to] = "Failed"]
       /\ failedClosed' = [failedClosed EXCEPT ![msg.to] = TRUE]
       /\ failReason' = [failReason EXCEPT ![msg.to] = "Conflict"]
    /\ network' = network \ {msg}
    /\ conflictObserved' = TRUE
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, activeOp,
                   nextOp, completedOps, targetGeneration, lockedOp, exposed,
                   receipt, retainedStart, history, heldAt, reportedExposed,
                   reportedReceipt, reportedRetained, reportSeen, cut,
                   cutChosen, knownCut, backfillCount, readySeen, certified,
                   disconnected, frozenAt, commitCut, committedOp,
                   committedGeneration, opReportedExposed, opCommitCut,
                   eventCount, timeoutTicks, duplicateHandled, staleIgnored,
                   duplicateBudget>>

InjectConflict(dst, f, value) ==
    /\ DELIVERY_MODE = "ArbitraryLoss"
    /\ activeOp \in Ops
    /\ phase[dst] = "Prepared"
    /\ knownCut[dst] # NO_FRAME
    /\ f \in NeededFrames(dst)
    /\ value \in INPUTS
    /\ value # stream[f]
    /\ LET msg == BackfillMessage(activeOp, Other(dst), dst, knownCut[dst], f, value) IN
       /\ CanEnqueue(msg)
       /\ network' = network \union {msg}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, certified, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, duplicateBudget>>

MissingHistoryFailure(p) ==
    /\ knownCut[p] # NO_FRAME
    /\ MissingNeeded(p) # {}
    /\ \E f \in MissingNeeded(p): \A src \in PEERS: history[src][f] = NO_VALUE
    /\ BeginFailure(p, "MissingHistory")

OverBudgetFailure(p) ==
    /\ knownCut[p] # NO_FRAME
    /\ Cardinality(MissingNeeded(p)) > MAX_BACKFILL - backfillCount[p]
    /\ BeginFailure(p, "OverBudget")

LoseRetainedHistory(p, f) ==
    /\ DELIVERY_MODE = "ArbitraryLoss"
    /\ activeOp \in Ops
    /\ phase[p] = "Prepared"
    /\ p # COORD
    /\ knownCut[COORD] # NO_FRAME
    /\ f \in MissingNeeded(COORD)
    /\ history[p][f] # NO_VALUE
    /\ history' = [history EXCEPT ![p][f] = NO_VALUE]
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, heldAt, reportedExposed,
                   reportedReceipt, reportedRetained, reportSeen, cut,
                   cutChosen, knownCut, backfillCount, readySeen, certified,
                   disconnected, frozenAt, commitCut, committedOp,
                   committedGeneration, opReportedExposed, opCommitCut,
                   eventCount, failedClosed, failReason, timeoutTicks,
                   duplicateHandled, staleIgnored, conflictObserved, network,
                   duplicateBudget>>

ExhaustBackfillBudget(p) ==
    /\ DELIVERY_MODE = "ArbitraryLoss"
    /\ activeOp \in Ops
    /\ p = COORD
    /\ phase[p] = "Prepared"
    /\ knownCut[p] # NO_FRAME
    /\ MissingNeeded(p) # {}
    /\ backfillCount[p] < MAX_BACKFILL
    /\ backfillCount' = [backfillCount EXCEPT ![p] = MAX_BACKFILL]
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, readySeen, certified,
                   disconnected, frozenAt, commitCut, committedOp,
                   committedGeneration, opReportedExposed, opCommitCut,
                   eventCount, failedClosed, failReason, timeoutTicks,
                   duplicateHandled, staleIgnored, conflictObserved, network,
                   duplicateBudget>>

TickTimeout(p) ==
    /\ DELIVERY_MODE = "ArbitraryLoss"
    /\ activeOp \in Ops
    /\ phase[p] \in {"Prepared", "Ready"}
    /\ timeoutTicks[p] < MAX_TICKS
    /\ timeoutTicks' = [timeoutTicks EXCEPT ![p] = @ + 1]
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, certified, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   duplicateHandled, staleIgnored, conflictObserved, network,
                   duplicateBudget>>

TimeoutFailure(p) ==
    /\ DELIVERY_MODE = "ArbitraryLoss"
    /\ timeoutTicks[p] = MAX_TICKS
    /\ BeginFailure(p, "Timeout")

ParticipantLoss(p) ==
    /\ DELIVERY_MODE = "ArbitraryLoss"
    /\ phase[p] \in {"Prepared", "Ready"}
    /\ ~certified
    /\ BeginFailure(p, "ParticipantLoss")

GenerationChange(p) ==
    /\ DELIVERY_MODE = "ArbitraryLoss"
    /\ activeOp \in Ops
    /\ phase[p] \in {"Prepared", "Ready"}
    /\ targetGeneration[p] < MAX_GENERATION
    /\ targetGeneration' = [targetGeneration EXCEPT ![p] = @ + 1]
    /\ phase' = [phase EXCEPT ![p] = "Failed"]
    /\ failedClosed' = [failedClosed EXCEPT ![p] = TRUE]
    /\ failReason' = [failReason EXCEPT ![p] = "GenerationChanged"]
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, activeOp,
                   nextOp, completedOps, lockedOp, exposed, receipt,
                   retainedStart, history, heldAt, reportedExposed,
                   reportedReceipt, reportedRetained, reportSeen, cut,
                   cutChosen, knownCut, backfillCount, readySeen, certified,
                   disconnected, frozenAt, commitCut, committedOp,
                   committedGeneration, opReportedExposed, opCommitCut,
                   eventCount, timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, network, duplicateBudget>>

SendFail(src, dst) ==
    /\ activeOp \in Ops
    /\ phase[src] = "Failed"
    /\ phase[dst] \notin {"Committed", "Failed"}
    /\ CanEnqueue(FailMessage(activeOp, src, dst))
    /\ network' = network \union {FailMessage(activeOp, src, dst)}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, certified, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, duplicateBudget>>

DeliverFail(msg) ==
    /\ msg \in network
    /\ msg.kind = "Fail"
       /\ msg.op = activeOp
       /\ phase[msg.to] \notin {"Committed", "Failed"}
       /\ phase' = [phase EXCEPT ![msg.to] = "Failed"]
       /\ failedClosed' = [failedClosed EXCEPT ![msg.to] = TRUE]
       /\ failReason' = [failReason EXCEPT ![msg.to] = "ParticipantLoss"]
    /\ network' = network \ {msg}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, activeOp,
                   nextOp, completedOps, targetGeneration, lockedOp, exposed,
                   receipt, retainedStart, history, heldAt, reportedExposed,
                   reportedReceipt, reportedRetained, reportSeen, cut,
                   cutChosen, knownCut, backfillCount, readySeen, certified,
                   disconnected, frozenAt, commitCut, committedOp,
                   committedGeneration, opReportedExposed, opCommitCut,
                   eventCount, timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, duplicateBudget>>

MarkReady(p) ==
    /\ activeOp \in Ops
    /\ phase[p] = "Prepared"
    /\ lockedOp[p] = activeOp
    /\ knownCut[p] = cut
    /\ AllNeededCanonical(p)
    /\ phase' = [phase EXCEPT ![p] = "Ready"]
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, activeOp,
                   nextOp, completedOps, targetGeneration, lockedOp, exposed,
                   receipt, retainedStart, history, heldAt, reportedExposed,
                   reportedReceipt, reportedRetained, reportSeen, cut,
                   cutChosen, knownCut, backfillCount, readySeen, certified,
                   disconnected, frozenAt, commitCut, committedOp,
                   committedGeneration, opReportedExposed, opCommitCut,
                   eventCount, failedClosed, failReason, timeoutTicks,
                   duplicateHandled, staleIgnored, conflictObserved, network,
                   duplicateBudget>>

SendReady(p) ==
    /\ activeOp \in Ops
    /\ phase[p] = "Ready"
    /\ p \notin readySeen
    /\ CanEnqueue(ReadyMessage(activeOp, p, cut))
    /\ network' = network \union {ReadyMessage(activeOp, p, cut)}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, certified, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, duplicateBudget>>

DeliverReady(msg) ==
    /\ msg \in network
    /\ msg.kind = "Ready"
       /\ msg.op = activeOp
       /\ msg.generation = OpGeneration(activeOp)
       /\ msg.cut = cut
       /\ phase[msg.from] = "Ready"
       /\ readySeen' = readySeen \union {msg.from}
    /\ network' = network \ {msg}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   certified, disconnected, frozenAt, commitCut, committedOp,
                   committedGeneration, opReportedExposed, opCommitCut,
                   eventCount, failedClosed, failReason, timeoutTicks,
                   duplicateHandled, staleIgnored, conflictObserved,
                   duplicateBudget>>

Certify ==
    /\ activeOp \in Ops
    /\ readySeen = PEERS
    /\ \A p \in PEERS: phase[p] = "Ready" /\ lockedOp[p] = activeOp
    /\ ~certified
    /\ certified' = TRUE
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, disconnected, frozenAt, commitCut, committedOp,
                   committedGeneration, opReportedExposed, opCommitCut,
                   eventCount, failedClosed, failReason, timeoutTicks,
                   duplicateHandled, staleIgnored, conflictObserved, network,
                   duplicateBudget>>

SendCommit(from, to) ==
    /\ activeOp \in Ops
    /\ certified
    /\ from = COORD \/ phase[from] = "Committed"
    /\ phase[to] = "Ready"
    /\ CanEnqueue(CommitMessage(activeOp, from, to, cut))
    /\ network' = network \union {CommitMessage(activeOp, from, to, cut)}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, certified, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, duplicateBudget>>

DeliverCommit(msg) ==
    /\ msg \in network
    /\ msg.kind = "Commit"
       /\ msg.op = activeOp
       /\ msg.generation = targetGeneration[msg.to]
       /\ msg.cut = cut
       /\ certified
       /\ phase[msg.to] = "Ready"
       /\ lockedOp[msg.to] = msg.op
       /\ phase' = [phase EXCEPT ![msg.to] = "Committed"]
       /\ disconnected' = [disconnected EXCEPT ![msg.to] = TRUE]
       /\ frozenAt' = [frozenAt EXCEPT ![msg.to] = msg.cut]
       /\ commitCut' = [commitCut EXCEPT ![msg.to] = msg.cut]
       /\ committedOp' = [committedOp EXCEPT ![msg.to] = msg.op]
       /\ committedGeneration' = [committedGeneration EXCEPT
              ![msg.to] = msg.generation]
       /\ opCommitCut' = [opCommitCut EXCEPT ![msg.op][msg.to] = msg.cut]
       /\ eventCount' = [eventCount EXCEPT ![msg.to] = @ + 1]
    \* Retain one equivalent copy so duplicate commit reception is explicit.
    /\ UNCHANGED network
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, activeOp,
                   nextOp, completedOps, targetGeneration, lockedOp, exposed,
                   receipt, retainedStart, history, heldAt, reportedExposed,
                   reportedReceipt, reportedRetained, reportSeen, cut,
                   cutChosen, knownCut, backfillCount, readySeen, certified,
                   opReportedExposed, failedClosed, failReason, timeoutTicks,
                   duplicateHandled, staleIgnored, conflictObserved,
                   duplicateBudget>>

DeliverDuplicateCommit(msg) ==
    /\ msg \in network
    /\ msg.kind = "Commit"
       /\ msg.op = 2
       /\ duplicateBudget[msg.op]
       /\ msg.op = committedOp[msg.to]
       /\ msg.generation = committedGeneration[msg.to]
       /\ msg.cut = commitCut[msg.to]
       /\ phase[msg.to] = "Committed"
       /\ duplicateHandled' = [duplicateHandled EXCEPT ![msg.to] = TRUE]
       /\ duplicateBudget' = [duplicateBudget EXCEPT ![msg.op] = FALSE]
    /\ network' = network \ {msg}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, certified, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   timeoutTicks, staleIgnored, conflictObserved>>

FinishOperation ==
    /\ activeOp \in Ops
    /\ \A p \in PEERS: phase[p] = "Committed"
    /\ completedOps' = completedOps \union {activeOp}
    /\ nextOp' = activeOp + 1
    /\ activeOp' = NO_OP
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   targetGeneration, lockedOp, exposed, receipt, retainedStart,
                   history, heldAt, reportedExposed, reportedReceipt,
                   reportedRetained, reportSeen, cut, cutChosen, knownCut,
                   backfillCount, readySeen, certified, disconnected, frozenAt,
                   commitCut, committedOp, committedGeneration,
                   opReportedExposed, opCommitCut, eventCount, failedClosed,
                   failReason, timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, network, duplicateBudget>>

RebaseNextMembershipEra ==
    \* A prepare for the next intent may have been deferred while operation 1
    \* was closing. The runtime discards its old participant vector and
    \* re-derives it in the new membership era; this bounded one-stream model
    \* represents that atomic rebase together with target reactivation.
    /\ activeOp = NO_OP
    /\ nextOp = 2
    /\ completedOps = {1}
    /\ \A p \in PEERS: phase[p] = "Committed"
    /\ phase' = [p \in PEERS |-> "Running"]
    /\ targetGeneration' = [p \in PEERS |-> 1]
    /\ lockedOp' = [p \in PEERS |-> NO_OP]
    /\ exposed' = baseExposed
    /\ receipt' = baseReceipt
    /\ retainedStart' = baseRetained
    /\ history' = [p \in PEERS |->
          HistoryFor(baseRetained[p], baseReceipt[p], stream)]
    /\ disconnected' = [p \in PEERS |-> FALSE]
    /\ frozenAt' = [p \in PEERS |-> NO_FRAME]
    /\ commitCut' = [p \in PEERS |-> NO_FRAME]
    /\ eventCount' = [p \in PEERS |-> 1]
    /\ failedClosed' = [p \in PEERS |-> FALSE]
    /\ failReason' = [p \in PEERS |-> "None"]
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, activeOp,
                   nextOp, completedOps, heldAt, reportedExposed,
                   reportedReceipt, reportedRetained, reportSeen, cut,
                   cutChosen, knownCut, backfillCount, readySeen, certified,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, network, duplicateBudget>>

ReceiveStale(msg) ==
    /\ msg \in network
    /\ (msg.generation < targetGeneration[msg.to]
        \/ msg.era < CurrentMembershipEra)
    /\ staleIgnored' = [staleIgnored EXCEPT ![msg.to] = TRUE]
    /\ network' = network \ {msg}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, certified, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   timeoutTicks, duplicateHandled, conflictObserved,
                   duplicateBudget>>

DropMessage(msg) ==
    /\ DELIVERY_MODE = "ArbitraryLoss"
    /\ msg \in network
    /\ network' = network \ {msg}
    /\ UNCHANGED <<stream, baseReceipt, baseExposed, baseRetained, phase,
                   activeOp, nextOp, completedOps, targetGeneration, lockedOp,
                   exposed, receipt, retainedStart, history, heldAt,
                   reportedExposed, reportedReceipt, reportedRetained,
                   reportSeen, cut, cutChosen, knownCut, backfillCount,
                   readySeen, certified, disconnected, frozenAt, commitCut,
                   committedOp, committedGeneration, opReportedExposed,
                   opCommitCut, eventCount, failedClosed, failReason,
                   timeoutTicks, duplicateHandled, staleIgnored,
                   conflictObserved, duplicateBudget>>

Next ==
    \/ \E p \in PEERS: Advance(p)
    \/ Start
    \/ \E p \in PEERS: SendPrepare(p)
    \/ \E msg \in network: DeliverPrepare(msg)
    \/ \E p \in PEERS: SendReport(p)
    \/ \E msg \in network: DeliverReport(msg)
    \/ ChooseCut
    \/ \E p \in PEERS: SendCut(p)
    \/ \E msg \in network: DeliverCut(msg)
    \/ \E src, dst \in PEERS, f \in Frames: SendBackfill(src, dst, f)
    \/ \E msg \in network: DeliverGoodBackfill(msg)
    \/ \E msg \in network: DeliverDuplicateBackfill(msg)
    \/ \E msg \in network: DeliverConflictingBackfill(msg)
    \/ \E dst \in PEERS, f \in Frames, value \in INPUTS:
           InjectConflict(dst, f, value)
    \/ \E p \in PEERS: MissingHistoryFailure(p)
    \/ \E p \in PEERS: OverBudgetFailure(p)
    \/ \E p \in PEERS, f \in Frames: LoseRetainedHistory(p, f)
    \/ \E p \in PEERS: ExhaustBackfillBudget(p)
    \/ \E p \in PEERS: TickTimeout(p)
    \/ \E p \in PEERS: TimeoutFailure(p)
    \/ \E p \in PEERS: ParticipantLoss(p)
    \/ \E p \in PEERS: GenerationChange(p)
    \/ \E src, dst \in PEERS: SendFail(src, dst)
    \/ \E msg \in network: DeliverFail(msg)
    \/ \E p \in PEERS: MarkReady(p)
    \/ \E p \in PEERS: SendReady(p)
    \/ \E msg \in network: DeliverReady(msg)
    \/ Certify
    \/ \E from, to \in PEERS: SendCommit(from, to)
    \/ \E msg \in network: DeliverCommit(msg)
    \/ \E msg \in network: DeliverDuplicateCommit(msg)
    \/ FinishOperation
    \/ RebaseNextMembershipEra
    \/ \E msg \in network: ReceiveStale(msg)
    \/ \E msg \in network: DropMessage(msg)

Spec == Init /\ [][Next]_vars

(***************************************************************************)
(* Fault companion. It runs one operation through prepare/report/cut, then *)
(* exercises one of the explicit fail-closed causes and propagates Fail to *)
(* every survivor. Strong fairness is limited to the cause/propagation     *)
(* unions; arbitrary message loss remains enabled but cannot starve both.  *)
(***************************************************************************)
FaultCause ==
    \/ \E i \in network: DeliverConflictingBackfill(i)
    \/ \E dst \in PEERS, f \in Frames, value \in INPUTS:
           InjectConflict(dst, f, value)
    \/ \E p \in PEERS: MissingHistoryFailure(p)
    \/ \E p \in PEERS: OverBudgetFailure(p)
    \/ \E p \in PEERS, f \in Frames: LoseRetainedHistory(p, f)
    \/ \E p \in PEERS: ExhaustBackfillBudget(p)
    \/ \E p \in PEERS: TickTimeout(p)
    \/ \E p \in PEERS: TimeoutFailure(p)
    \/ \E p \in PEERS: ParticipantLoss(p)
    \/ \E p \in PEERS: GenerationChange(p)

TerminalFaultCause ==
    \/ \E msg \in network: DeliverConflictingBackfill(msg)
    \/ \E p \in PEERS: MissingHistoryFailure(p)
    \/ \E p \in PEERS: OverBudgetFailure(p)
    \/ \E p \in PEERS: TimeoutFailure(p)
    \/ \E p \in PEERS: ParticipantLoss(p)
    \/ \E p \in PEERS: GenerationChange(p)

FaultPropagation ==
    \/ \E src, dst \in PEERS: SendFail(src, dst)
    \/ \E msg \in network: DeliverFail(msg)

FaultNext ==
    \/ Start
    \/ \E p \in PEERS: SendPrepare(p)
    \/ \E msg \in network: DeliverPrepare(msg)
    \/ \E p \in PEERS: SendReport(p)
    \/ \E msg \in network: DeliverReport(msg)
    \/ ChooseCut
    \/ \E p \in PEERS: SendCut(p)
    \/ \E msg \in network: DeliverCut(msg)
    \/ FaultCause
    \/ FaultPropagation
    \/ \E msg \in network: DropMessage(msg)

FaultFairness ==
    /\ WF_vars(FaultNext)
    /\ SF_vars(TerminalFaultCause)
    /\ SF_vars(FaultPropagation)
    /\ SF_vars(\E msg \in network: DeliverFail(msg))

FaultSpec == Init /\ [][FaultNext]_vars /\ FaultFairness

(***************************************************************************)
(* Fair-delivery companion: loss/conflict/participant faults are disabled *)
(* by DELIVERY_MODE. Weak fairness covers every required protocol step;    *)
(* duplicate injection is not required for resolution.                     *)
(***************************************************************************)
Fairness ==
    WF_vars(Next)

FairSpec == Init /\ [][Next]_vars /\ Fairness

TypeInvariant ==
    /\ stream \in [Frames -> INPUTS]
    /\ baseReceipt \in [PEERS -> Frames]
    /\ baseExposed \in [PEERS -> Frames]
    /\ baseRetained \in [PEERS -> Frames]
    /\ phase \in [PEERS -> PeerPhases]
    /\ activeOp \in OpOrNone
    /\ nextOp \in 1..3
    /\ completedOps \subseteq Ops
    /\ targetGeneration \in [PEERS -> Generations]
    /\ lockedOp \in [PEERS -> OpOrNone]
    /\ exposed \in [PEERS -> Frames]
    /\ receipt \in [PEERS -> Frames]
    /\ retainedStart \in [PEERS -> Frames]
    /\ history \in [PEERS -> [Frames -> ValueOrNone]]
    /\ heldAt \in [PEERS -> FrameOrNone]
    /\ reportedExposed \in [PEERS -> FrameOrNone]
    /\ reportedReceipt \in [PEERS -> FrameOrNone]
    /\ reportedRetained \in [PEERS -> FrameOrNone]
    /\ reportSeen \subseteq PEERS
    /\ cut \in FrameOrNone
    /\ cutChosen \in BOOLEAN
    /\ knownCut \in [PEERS -> FrameOrNone]
    /\ backfillCount \in [PEERS -> 0..MAX_BACKFILL]
    /\ readySeen \subseteq PEERS
    /\ certified \in BOOLEAN
    /\ disconnected \in [PEERS -> BOOLEAN]
    /\ frozenAt \in [PEERS -> FrameOrNone]
    /\ commitCut \in [PEERS -> FrameOrNone]
    /\ committedOp \in [PEERS -> OpOrNone]
    /\ committedGeneration \in [PEERS -> FrameOrNone]
    /\ opReportedExposed \in [Ops -> [PEERS -> FrameOrNone]]
    /\ opCommitCut \in [Ops -> [PEERS -> FrameOrNone]]
    /\ eventCount \in [PEERS -> 0..2]
    /\ failedClosed \in [PEERS -> BOOLEAN]
    /\ failReason \in [PEERS -> FailReasons]
    /\ timeoutTicks \in [PEERS -> 0..MAX_TICKS]
    /\ duplicateHandled \in [PEERS -> BOOLEAN]
    /\ staleIgnored \in [PEERS -> BOOLEAN]
    /\ conflictObserved \in BOOLEAN
    /\ network \subseteq Message
    /\ Cardinality(network) <= MAX_NETWORK
    /\ duplicateBudget \in [Ops -> BOOLEAN]

PrepareHoldsConfirmation ==
    \A p \in PEERS:
        phase[p] \in {"Prepared", "Ready"} =>
            heldAt[p] = reportedExposed[p] /\ exposed[p] = heldAt[p]

CutNonRetracting ==
    cutChosen /\ FIX_MODE = "Barrier" =>
        /\ \A p \in PEERS: cut >= reportedExposed[p]
        /\ \A p \in PEERS: cut >= reportedReceipt[p]

OpReportedValue(op, p, f) ==
    IF opCommitCut[op][p] # NO_FRAME /\ f > opCommitCut[op][p]
    THEN stream[opCommitCut[op][p]]
    ELSE stream[f]

ConfirmedHistoryImmutable ==
    \A op \in Ops, p \in PEERS:
        opReportedExposed[op][p] # NO_FRAME =>
            \A f \in 0..opReportedExposed[op][p]:
                OpReportedValue(op, p, f) = stream[f]

CommitRequiresCertificate ==
    \A p \in PEERS:
        phase[p] = "Committed" /\ activeOp \in Ops =>
            certified /\ readySeen = PEERS /\ committedOp[p] = activeOp

CommitAgreement ==
    /\ \A op \in Ops, p, q \in PEERS:
           opCommitCut[op][p] # NO_FRAME /\ opCommitCut[op][q] # NO_FRAME =>
               opCommitCut[op][p] = opCommitCut[op][q]
    /\ \A op \in completedOps, p \in PEERS:
           opCommitCut[op][p] # NO_FRAME

LocalCommitAtomic ==
    \A p \in PEERS:
        phase[p] = "Committed" =>
            disconnected[p] /\ frozenAt[p] = commitCut[p]
            /\ commitCut[p] # NO_FRAME /\ committedOp[p] # NO_OP

PeerDroppedExactlyOncePerCommit ==
    \A p \in PEERS: eventCount[p] = Cardinality(completedOps)
        \/ (activeOp \in Ops /\ phase[p] = "Committed"
            /\ eventCount[p] = Cardinality(completedOps) + 1)

GenerationFence ==
    \A p \in PEERS:
        phase[p] = "Committed" =>
            committedGeneration[p] = OpGeneration(committedOp[p])

FailureIsSticky ==
    \A p \in PEERS:
        /\ failedClosed[p] <=> phase[p] = "Failed"
        /\ failedClosed[p] <=> failReason[p] # "None"

ConflictFailsClosed ==
    conflictObserved =>
        \E p \in PEERS: phase[p] = "Failed" /\ failReason[p] = "Conflict"

ReadyHasCanonicalRetainedPrefix ==
    \A p \in PEERS: phase[p] = "Ready" => AllNeededCanonical(p)

BackfillBounded == \A p \in PEERS: backfillCount[p] <= MAX_BACKFILL

SerializedOperations ==
    /\ activeOp = 2 => 1 \in completedOps
    /\ 2 \in completedOps => 1 \in completedOps
    /\ Cardinality(completedOps) <= 2

SerializedMembershipEra ==
    /\ activeOp = 2 =>
           1 \in completedOps /\ CurrentMembershipEra = OperationEra(2)
    /\ (\A p \in PEERS: phase[p] # "Failed") =>
           CurrentMembershipEra \in {0, 1}
    /\ CurrentMembershipEra = 1 /\ (\A p \in PEERS: phase[p] # "Failed") =>
           1 \in completedOps

SafetyInvariant ==
    /\ TypeInvariant
    /\ PrepareHoldsConfirmation
    /\ CutNonRetracting
    /\ ConfirmedHistoryImmutable
    /\ CommitRequiresCertificate
    /\ CommitAgreement
    /\ LocalCommitAtomic
    /\ PeerDroppedExactlyOncePerCommit
    /\ GenerationFence
    /\ FailureIsSticky
    /\ ConflictFailsClosed
    /\ ReadyHasCanonicalRetainedPrefix
    /\ BackfillBounded
    /\ SerializedOperations
    /\ SerializedMembershipEra

EventuallyResolvedTwoOps ==
    <> (nextOp = 3 /\ completedOps = Ops /\
        \A p \in PEERS: phase[p] = "Committed")

EventuallyBackfilled == <> (\E p \in PEERS: backfillCount[p] > 0)
EventuallyReceiptRaisesCut ==
    <> (cutChosen /\ cut > MaxOf({reportedExposed[p]: p \in PEERS}))
EventuallyMembershipEraRebased ==
    <> (activeOp = 2 /\ CurrentMembershipEra = OperationEra(2))
EventuallyStaleIgnored == <> (\E p \in PEERS: staleIgnored[p])
EventuallyDuplicateHandled == <> (\E p \in PEERS: duplicateHandled[p])
EventuallyAllFailed == <> (\A p \in PEERS: phase[p] = "Failed")
PreparedImpliesEventuallyAllFailed ==
    []( (\E p \in PEERS: phase[p] \in {"Prepared", "Ready"}) =>
        <> (\A p \in PEERS: phase[p] = "Failed") )

THEOREM Spec => []SafetyInvariant

=============================================================================
