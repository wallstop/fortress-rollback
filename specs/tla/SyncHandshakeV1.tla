-------------------------- MODULE SyncHandshakeV1 --------------------------
(***************************************************************************)
(* Fortress Rollback protocol-v1 configuration handshake.                  *)
(*                                                                         *)
(* The arbitrary-loss model checks safety; SyncHandshakeV1Fair separately  *)
(* checks success under fair delivery. Timeout is event-only: reporting it  *)
(* leaves the endpoint Syncing and requests remain enabled. Incompatible    *)
(* configuration is the only terminal failure modeled here.                *)
(*                                                                         *)
(* Only numPlayers and inputWidth are represented. They exercise the same   *)
(* ordered field comparison planned for the full production config block.   *)
(* Rust tests remain responsible for the omitted fields and public event    *)
(* translation; see specs/tla/README.md.                                    *)
(***************************************************************************)

EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    PEERS,
    PLAYER_COUNTS,
    INPUT_WIDTHS,
    NUM_SYNC_PACKETS,
    TIMEOUT_TICKS,
    MAX_NETWORK,
    DELIVERY_MODE,
    CONFIG_MODE,
    NO_CONFIG,
    NO_PEER,
    NO_VALUE

ASSUME Cardinality(PEERS) = 2
ASSUME PLAYER_COUNTS # {}
ASSUME INPUT_WIDTHS # {}
ASSUME NUM_SYNC_PACKETS \in Nat /\ NUM_SYNC_PACKETS > 0
ASSUME TIMEOUT_TICKS \in Nat /\ TIMEOUT_TICKS > 0
ASSUME MAX_NETWORK \in Nat /\ MAX_NETWORK >= Cardinality(PEERS)
ASSUME DELIVERY_MODE \in {"ArbitraryLoss", "FairDelivery", "HandlersDisabled"}
ASSUME CONFIG_MODE \in {"All", "Matching", "Mismatching"}

Phases == {"Syncing", "Synced", "Failed"}
MessageKinds == {"SyncRequest", "SyncReply"}
ReasonFields == {"None", "NumPlayers", "InputWidth"}
Configs == [numPlayers: PLAYER_COUNTS, inputWidth: INPUT_WIDTHS]
ConfigValues == PLAYER_COUNTS \union INPUT_WIDTHS
Tokens == 1..NUM_SYNC_PACKETS

ASSUME NO_CONFIG \notin Configs
ASSUME NO_PEER \notin PEERS
ASSUME NO_VALUE \notin ConfigValues

Other(p) == CHOOSE q \in PEERS: q # p
FirstPeer == CHOOSE p \in PEERS: TRUE

Message == [
    kind: MessageKinds,
    from: PEERS,
    to: PEERS,
    token: Tokens,
    config: Configs
]

RemoveAt(seq, index) ==
    SubSeq(seq, 1, index - 1) \o SubSeq(seq, index + 1, Len(seq))

NextToken(token) == IF token = NUM_SYNC_PACKETS THEN 1 ELSE token + 1

MismatchField(ours, theirs) ==
    IF ours.numPlayers # theirs.numPlayers THEN "NumPlayers"
    ELSE IF ours.inputWidth # theirs.inputWidth THEN "InputWidth"
    ELSE "None"

MismatchOurs(ours, theirs) ==
    IF ours.numPlayers # theirs.numPlayers THEN ours.numPlayers
    ELSE IF ours.inputWidth # theirs.inputWidth THEN ours.inputWidth
    ELSE NO_VALUE

MismatchTheirs(ours, theirs) ==
    IF ours.numPlayers # theirs.numPlayers THEN theirs.numPlayers
    ELSE IF ours.inputWidth # theirs.inputWidth THEN theirs.inputWidth
    ELSE NO_VALUE

VARIABLES
    phase,
    localConfig,
    learnedConfig,
    learnedFrom,
    syncRemaining,
    acceptedTokens,
    nextToken,
    timeoutTicks,
    timeoutEventCount,
    incompatibleEventCount,
    reasonField,
    reasonOurs,
    reasonTheirs,
    network

vars == <<
    phase, localConfig, learnedConfig, learnedFrom, syncRemaining,
    acceptedTokens, nextToken, timeoutTicks, timeoutEventCount,
    incompatibleEventCount, reasonField, reasonOurs, reasonTheirs, network
>>

TypeInvariant ==
    /\ phase \in [PEERS -> Phases]
    /\ localConfig \in [PEERS -> Configs]
    /\ learnedConfig \in [PEERS -> (Configs \union {NO_CONFIG})]
    /\ learnedFrom \in [PEERS -> (PEERS \union {NO_PEER})]
    /\ syncRemaining \in [PEERS -> 0..NUM_SYNC_PACKETS]
    /\ acceptedTokens \in [PEERS -> SUBSET Tokens]
    /\ nextToken \in [PEERS -> Tokens]
    /\ timeoutTicks \in [PEERS -> 0..TIMEOUT_TICKS]
    /\ timeoutEventCount \in [PEERS -> 0..1]
    /\ incompatibleEventCount \in [PEERS -> 0..1]
    /\ reasonField \in [PEERS -> ReasonFields]
    /\ reasonOurs \in [PEERS -> (ConfigValues \union {NO_VALUE})]
    /\ reasonTheirs \in [PEERS -> (ConfigValues \union {NO_VALUE})]
    /\ network \in Seq(Message)
    /\ Len(network) <= MAX_NETWORK

Init ==
    /\ phase = [p \in PEERS |-> "Syncing"]
    /\ CASE CONFIG_MODE = "Matching" ->
                \E config \in Configs: localConfig = [p \in PEERS |-> config]
            [] CONFIG_MODE = "Mismatching" ->
                \E configA, configB \in Configs:
                    /\ configA # configB
                    /\ localConfig = [p \in PEERS |->
                           IF p = FirstPeer THEN configA ELSE configB]
            [] OTHER -> localConfig \in [PEERS -> Configs]
    /\ learnedConfig = [p \in PEERS |-> NO_CONFIG]
    /\ learnedFrom = [p \in PEERS |-> NO_PEER]
    /\ syncRemaining = [p \in PEERS |-> NUM_SYNC_PACKETS]
    /\ acceptedTokens = [p \in PEERS |-> {}]
    /\ nextToken = [p \in PEERS |-> 1]
    /\ timeoutTicks = [p \in PEERS |-> 0]
    /\ timeoutEventCount = [p \in PEERS |-> 0]
    /\ incompatibleEventCount = [p \in PEERS |-> 0]
    /\ reasonField = [p \in PEERS |-> "None"]
    /\ reasonOurs = [p \in PEERS |-> NO_VALUE]
    /\ reasonTheirs = [p \in PEERS |-> NO_VALUE]
    /\ network = <<>>

(***************************************************************************)
(* Requests may overlap and token order may wrap before replies arrive.    *)
(* acceptedTokens makes reordered and duplicate-token replies idempotent.   *)
(***************************************************************************)
SendSyncRequest(p) ==
    /\ phase[p] = "Syncing"
    /\ Len(network) < MAX_NETWORK
    /\ network' = Append(network, [
           kind |-> "SyncRequest",
           from |-> p,
           to |-> Other(p),
           token |-> nextToken[p],
           config |-> localConfig[p]
       ])
    /\ nextToken' = [nextToken EXCEPT ![p] = NextToken(@)]
    /\ UNCHANGED <<
        phase, localConfig, learnedConfig, learnedFrom, syncRemaining,
        acceptedTokens, timeoutTicks, timeoutEventCount,
        incompatibleEventCount, reasonField, reasonOurs, reasonTheirs
       >>

HandleSyncRequest(p) ==
    /\ DELIVERY_MODE # "HandlersDisabled"
    /\ \E index \in 1..Len(network):
        /\ network[index].kind = "SyncRequest"
        /\ network[index].to = p
        /\ LET msg == network[index] IN
           /\ network' = Append(RemoveAt(network, index), [
                  kind |-> "SyncReply", from |-> p, to |-> msg.from,
                  token |-> msg.token, config |-> localConfig[p]
              ])
           /\ learnedConfig' = [learnedConfig EXCEPT ![p] = msg.config]
           /\ learnedFrom' = [learnedFrom EXCEPT ![p] = msg.from]
           /\ IF msg.config # localConfig[p] /\ phase[p] = "Syncing"
              THEN
                /\ phase' = [phase EXCEPT ![p] = "Failed"]
                /\ incompatibleEventCount' = [incompatibleEventCount EXCEPT ![p] = 1]
                /\ reasonField' = [reasonField EXCEPT
                       ![p] = MismatchField(localConfig[p], msg.config)]
                /\ reasonOurs' = [reasonOurs EXCEPT
                       ![p] = MismatchOurs(localConfig[p], msg.config)]
                /\ reasonTheirs' = [reasonTheirs EXCEPT
                       ![p] = MismatchTheirs(localConfig[p], msg.config)]
              ELSE UNCHANGED <<
                       phase, incompatibleEventCount, reasonField,
                       reasonOurs, reasonTheirs
                   >>
    /\ UNCHANGED <<
        localConfig, syncRemaining, acceptedTokens, nextToken, timeoutTicks,
        timeoutEventCount
       >>

HandleSyncReply(p) ==
    /\ DELIVERY_MODE # "HandlersDisabled"
    /\ \E index \in 1..Len(network):
        /\ network[index].kind = "SyncReply"
        /\ network[index].to = p
        /\ LET msg == network[index] IN
           /\ network' = RemoveAt(network, index)
           /\ IF phase[p] # "Syncing" \/ msg.token \in acceptedTokens[p]
              THEN UNCHANGED <<
                       phase, learnedConfig, learnedFrom, syncRemaining,
                       acceptedTokens, incompatibleEventCount, reasonField,
                       reasonOurs, reasonTheirs
                   >>
              ELSE
                /\ learnedConfig' = [learnedConfig EXCEPT ![p] = msg.config]
                /\ learnedFrom' = [learnedFrom EXCEPT ![p] = msg.from]
                /\ IF msg.config # localConfig[p]
                   THEN
                     /\ phase' = [phase EXCEPT ![p] = "Failed"]
                     /\ incompatibleEventCount' = [incompatibleEventCount EXCEPT ![p] = 1]
                     /\ reasonField' = [reasonField EXCEPT
                            ![p] = MismatchField(localConfig[p], msg.config)]
                     /\ reasonOurs' = [reasonOurs EXCEPT
                            ![p] = MismatchOurs(localConfig[p], msg.config)]
                     /\ reasonTheirs' = [reasonTheirs EXCEPT
                            ![p] = MismatchTheirs(localConfig[p], msg.config)]
                     /\ UNCHANGED <<syncRemaining, acceptedTokens>>
                   ELSE
                     /\ acceptedTokens' = [acceptedTokens EXCEPT ![p] = @ \union {msg.token}]
                     /\ syncRemaining' = [syncRemaining EXCEPT ![p] = @ - 1]
                     /\ phase' = IF syncRemaining[p] = 1
                                  THEN [phase EXCEPT ![p] = "Synced"]
                                  ELSE phase
                     /\ UNCHANGED <<
                          incompatibleEventCount, reasonField,
                          reasonOurs, reasonTheirs
                         >>
    /\ UNCHANGED <<localConfig, nextToken, timeoutTicks, timeoutEventCount>>

DropMessage ==
    /\ DELIVERY_MODE = "ArbitraryLoss"
    /\ Len(network) > 0
    /\ \E index \in 1..Len(network): network' = RemoveAt(network, index)
    /\ UNCHANGED <<
        phase, localConfig, learnedConfig, learnedFrom, syncRemaining,
        acceptedTokens, nextToken, timeoutTicks, timeoutEventCount,
        incompatibleEventCount, reasonField, reasonOurs, reasonTheirs
       >>

TickTimeout(p) ==
    /\ phase[p] = "Syncing"
    /\ timeoutTicks[p] < TIMEOUT_TICKS
    /\ timeoutTicks' = [timeoutTicks EXCEPT ![p] = @ + 1]
    /\ UNCHANGED <<
        phase, localConfig, learnedConfig, learnedFrom, syncRemaining,
        acceptedTokens, nextToken, timeoutEventCount, incompatibleEventCount,
        reasonField, reasonOurs, reasonTheirs, network
       >>

ReportSyncTimeout(p) ==
    /\ phase[p] = "Syncing"
    /\ timeoutTicks[p] = TIMEOUT_TICKS
    /\ timeoutEventCount[p] = 0
    /\ timeoutEventCount' = [timeoutEventCount EXCEPT ![p] = 1]
    /\ UNCHANGED <<
        phase, localConfig, learnedConfig, learnedFrom, syncRemaining,
        acceptedTokens, nextToken, timeoutTicks, incompatibleEventCount,
        reasonField, reasonOurs, reasonTheirs, network
       >>

MutationIdle ==
    /\ DELIVERY_MODE = "HandlersDisabled"
    /\ UNCHANGED vars

Next ==
    \/ \E p \in PEERS: SendSyncRequest(p)
    \/ \E p \in PEERS: HandleSyncRequest(p)
    \/ \E p \in PEERS: HandleSyncReply(p)
    \/ DropMessage
    \/ \E p \in PEERS: TickTimeout(p)
    \/ \E p \in PEERS: ReportSyncTimeout(p)
    \/ MutationIdle

SafetySpec == Init /\ [][Next]_vars

Fairness ==
    /\ \A p \in PEERS: WF_vars(SendSyncRequest(p))
    /\ \A p \in PEERS: WF_vars(HandleSyncRequest(p))
    /\ \A p \in PEERS: WF_vars(HandleSyncReply(p))
    /\ \A p \in PEERS: WF_vars(TickTimeout(p))
    /\ \A p \in PEERS: WF_vars(ReportSyncTimeout(p))

FairSpec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety                                                                  *)
(***************************************************************************)

LearnedConfigAuthentic ==
    \A p \in PEERS:
        learnedConfig[p] # NO_CONFIG =>
            /\ learnedFrom[p] = Other(p)
            /\ learnedConfig[p] = localConfig[learnedFrom[p]]

SyncedOnlyWithMatchingConfig ==
    \A p \in PEERS:
        phase[p] = "Synced" =>
            /\ learnedFrom[p] = Other(p)
            /\ learnedConfig[p] = localConfig[p]
            /\ localConfig[p] = localConfig[Other(p)]
            /\ syncRemaining[p] = 0

MismatchNeverSynchronizes ==
    (\E p \in PEERS: localConfig[p] # localConfig[Other(p)]) =>
        \A p \in PEERS: phase[p] # "Synced"

AcceptedTokensExact ==
    \A p \in PEERS:
        Cardinality(acceptedTokens[p]) = NUM_SYNC_PACKETS - syncRemaining[p]

IncompatibleEventExactlyOnce ==
    \A p \in PEERS:
        /\ (phase[p] = "Failed") <=> (incompatibleEventCount[p] = 1)
        /\ incompatibleEventCount[p] = 0 => reasonField[p] = "None"
        /\ incompatibleEventCount[p] = 0 => reasonOurs[p] = NO_VALUE
        /\ incompatibleEventCount[p] = 0 => reasonTheirs[p] = NO_VALUE

IncompatibleReasonOriented ==
    \A p \in PEERS:
        incompatibleEventCount[p] = 1 =>
            /\ reasonField[p] = MismatchField(localConfig[p], learnedConfig[p])
            /\ reasonOurs[p] = MismatchOurs(localConfig[p], learnedConfig[p])
            /\ reasonTheirs[p] = MismatchTheirs(localConfig[p], learnedConfig[p])
            /\ reasonField[p] # "None"

ObservedMismatchFailsAndReports ==
    \A p \in PEERS:
        learnedConfig[p] # NO_CONFIG /\ learnedConfig[p] # localConfig[p] =>
            /\ phase[p] = "Failed"
            /\ incompatibleEventCount[p] = 1

FailureIsSticky ==
    [][\A p \in PEERS:
        phase[p] = "Failed" => phase'[p] = "Failed"]_vars

TimeoutTransitionIsEventOnly ==
    [][\A p \in PEERS:
        timeoutEventCount[p] = 0 /\ timeoutEventCount'[p] = 1 =>
            /\ phase'[p] = phase[p]
            /\ syncRemaining'[p] = syncRemaining[p]
            /\ acceptedTokens'[p] = acceptedTokens[p]
            /\ nextToken'[p] = nextToken[p]]_vars

(***************************************************************************)
(* Fair-delivery liveness                                                  *)
(***************************************************************************)

EventuallyBothSynced ==
    <> (\A p \in PEERS: phase[p] = "Synced")

EventuallyBothFailed ==
    <> (\A p \in PEERS: phase[p] = "Failed")

=============================================================================
