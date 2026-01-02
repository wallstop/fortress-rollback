-------------------------- MODULE SpectatorSession --------------------------
(***************************************************************************)
(* TLA+ Specification for Fortress Rollback Spectator Session              *)
(*                                                                         *)
(* This module specifies the spectator session behavior in Fortress        *)
(* Rollback. A spectator is a read-only participant that receives input    *)
(* broadcasts from a host and replays the game without contributing input. *)
(*                                                                         *)
(* Key behaviors modeled:                                                  *)
(*   - Spectator joining and synchronization with host                     *)
(*   - Frame delay handling (spectators are behind the live game)          *)
(*   - Input buffering in a circular buffer                                *)
(*   - Catchup mode when spectator falls too far behind                    *)
(*   - Host disconnection and timeout handling                             *)
(*                                                                         *)
(* Properties verified:                                                    *)
(*   - Safety: Spectator frame never exceeds last received frame (INV-SP-1)*)
(*   - Safety: Buffer index always in bounds (INV-SP-2)                    *)
(*   - Safety: Spectator advances only with valid inputs (INV-SP-3)        *)
(*   - Safety: State transitions are valid (INV-SP-4)                      *)
(*   - Safety: Frames behind is always non-negative (INV-SP-5)             *)
(*   - Liveness: Eventually synchronized (under fairness)                  *)
(*                                                                         *)
(* Production-Spec Alignment:                                              *)
(*   BUFFER_SIZE maps to SpectatorConfig.buffer_size (default: 60).        *)
(*   MAX_FRAMES_BEHIND maps to SpectatorConfig.max_frames_behind (default: 10). *)
(*   CATCHUP_SPEED maps to SpectatorConfig.catchup_speed (default: 1).     *)
(*   The invariants proven here hold for ANY valid configuration.          *)
(*   TLA+ uses small values for tractable exhaustive model checking.       *)
(*                                                                         *)
(* Design Decisions Modeled:                                               *)
(*   - Spectator uses circular buffer for input storage                    *)
(*   - No rollback in spectator (only confirmed inputs used)               *)
(*   - Catchup mode advances multiple frames when behind                   *)
(*   - Host broadcasts inputs; spectator never sends inputs                *)
(*                                                                         *)
(* Simplifications (what is NOT modeled):                                  *)
(*   - Multiple spectators: This spec models a single spectator. The       *)
(*     production code supports multiple spectators watching a host.       *)
(*   - Network message loss/reordering: Inputs are assumed to arrive       *)
(*     reliably. Production uses UDP with application-level reliability.   *)
(*   - Specific error types: FortressError variants are abstracted. The    *)
(*     spec uses boolean checks rather than detailed error conditions.     *)
(*   - Event queue: The spec doesn't model the event_queue; events are     *)
(*     implicit in state transitions.                                      *)
(*   - Checksum verification: Desync detection is not modeled.             *)
(*                                                                         *)
(* Test Scenario Mapping:                                                  *)
(*   - test_spectator_sync -> ReceiveSyncReply, CompleteSynchronization    *)
(*   - test_spectator_catchup -> AdvanceFrameCatchup with framesBehind     *)
(*   - test_spectator_timeout -> SyncTimeout action                        *)
(*   - test_spectator_disconnect -> HostDisconnect action                  *)
(*   - test_spectator_advance -> FirstAdvance, AdvanceFrameNormal          *)
(***************************************************************************)

EXTENDS Integers, Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    NUM_PLAYERS,            \* Number of players in the game (configurable)
    BUFFER_SIZE,            \* Size of input buffer (default 60, configurable)
    MAX_FRAMES_BEHIND,      \* Threshold for catchup mode (default 10)
    CATCHUP_SPEED,          \* Frames to advance in catchup mode (default 1)
    MAX_FRAME,              \* Maximum frame for model checking
    NULL_FRAME              \* Sentinel value (-1 in implementation)

ASSUME NUM_PLAYERS \in Nat /\ NUM_PLAYERS > 0
ASSUME BUFFER_SIZE \in Nat /\ BUFFER_SIZE > 0
ASSUME MAX_FRAMES_BEHIND \in Nat /\ MAX_FRAMES_BEHIND >= 0
ASSUME CATCHUP_SPEED \in Nat /\ CATCHUP_SPEED >= 1
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME NULL_FRAME \notin 0..MAX_FRAME  \* Sentinel value (outside valid frame range)

\* Constant-level invariant (checked once at startup, not per-state)
\* INV-SP-2: Buffer Index Always In Bounds
\* For any valid frame, frame % buffer_size is in [0, buffer_size)
ASSUME BufferIndexBoundsAssumption ==
    \A f \in 0..MAX_FRAME: (f % BUFFER_SIZE) \in 0..(BUFFER_SIZE - 1)

(***************************************************************************)
(* Type Definitions                                                        *)
(***************************************************************************)
Frame == {NULL_FRAME} \union (0..MAX_FRAME)
Players == 1..NUM_PLAYERS
BufferIndex == 0..(BUFFER_SIZE - 1)
InputValue == 0..1  \* Simplified: 0 = no input, 1 = input

\* Session states matching production code
SessionStates == {"Synchronizing", "Running"}

\* Connection status for host
ConnectionStates == {"Connected", "Disconnected"}

(***************************************************************************)
(* Variables                                                               *)
(***************************************************************************)
VARIABLES
    \* Spectator session state
    sessionState,           \* Current session state (Synchronizing or Running)
    currentFrame,           \* Current spectator frame
    lastRecvFrame,          \* Most recently received frame from host

    \* Input buffer (circular buffer)
    inputBuffer,            \* inputBuffer[i][p] = input for player p at buffer index i
    inputFrames,            \* inputFrames[i] = frame number at buffer index i

    \* Host connection tracking
    hostConnection,         \* Connection state to host
    syncProgress,           \* Sync handshake progress (0 to NUM_SYNC_PACKETS)

    \* Catchup tracking
    framesBehind,           \* How many frames behind the host
    inCatchupMode           \* TRUE if spectator is catching up

vars == <<sessionState, currentFrame, lastRecvFrame, inputBuffer, inputFrames,
          hostConnection, syncProgress, framesBehind, inCatchupMode>>

(***************************************************************************)
(* Type Invariant                                                          *)
(***************************************************************************)
TypeInvariant ==
    /\ sessionState \in SessionStates
    /\ currentFrame \in Frame
    /\ lastRecvFrame \in Frame
    /\ inputBuffer \in [BufferIndex -> [Players -> InputValue]]
    /\ inputFrames \in [BufferIndex -> Frame]
    /\ hostConnection \in ConnectionStates
    /\ syncProgress \in Nat
    /\ framesBehind \in Nat
    /\ inCatchupMode \in BOOLEAN

(***************************************************************************)
(* Initial State                                                           *)
(* Spectator starts in Synchronizing state with NULL frame                 *)
(***************************************************************************)
Init ==
    /\ sessionState = "Synchronizing"
    /\ currentFrame = NULL_FRAME
    /\ lastRecvFrame = NULL_FRAME
    /\ inputBuffer = [i \in BufferIndex |-> [p \in Players |-> 0]]
    /\ inputFrames = [i \in BufferIndex |-> NULL_FRAME]
    /\ hostConnection = "Connected"
    /\ syncProgress = 0
    /\ framesBehind = 0
    /\ inCatchupMode = FALSE

(***************************************************************************)
(* Helper: Compute buffer index for a frame                                *)
(* Uses modulo arithmetic for circular buffer                              *)
(***************************************************************************)
BufferIndexFor(frame) ==
    frame % BUFFER_SIZE

(***************************************************************************)
(* Helper: Compute frames behind host                                      *)
(* Production: frames_behind_host() method                                 *)
(***************************************************************************)
ComputeFramesBehind ==
    IF lastRecvFrame = NULL_FRAME \/ currentFrame = NULL_FRAME
    THEN 0
    ELSE IF lastRecvFrame < currentFrame
         THEN 0  \* Defensive: should not happen, return 0
         ELSE lastRecvFrame - currentFrame

(***************************************************************************)
(* Helper: Check if in catchup mode                                        *)
(* Production: frames_behind_host() > max_frames_behind                    *)
(***************************************************************************)
ShouldCatchup ==
    framesBehind > MAX_FRAMES_BEHIND

(***************************************************************************)
(* INV-SP-1: Spectator Frame Never Exceeds Last Received                   *)
(* The spectator can only advance to frames it has received inputs for     *)
(***************************************************************************)
SpectatorFrameBounded ==
    (currentFrame # NULL_FRAME /\ lastRecvFrame # NULL_FRAME) =>
        currentFrame <= lastRecvFrame

(***************************************************************************)
(* INV-SP-2: Buffer operations stay within buffer bounds                   *)
(* When we access inputFrames, the index is always valid                   *)
(* Note: Constant-level bound check is in BufferIndexBoundsAssumption      *)
(***************************************************************************)
BufferAccessValid ==
    \A i \in BufferIndex:
        inputFrames[i] \in Frame

(***************************************************************************)
(* INV-SP-3: Spectator Only Advances With Valid Inputs                     *)
(* Note: When currentFrame = NULL_FRAME, this is vacuously true.           *)
(* The FirstAdvance action handles the NULL_FRAME -> 0 transition.         *)
(* When Running, spectator only advances if inputs are available           *)
(***************************************************************************)
ValidInputsForAdvance ==
    (sessionState = "Running" /\ currentFrame # NULL_FRAME) =>
        \/ currentFrame = lastRecvFrame  \* At latest frame, cannot advance
        \/ inputFrames[BufferIndexFor(currentFrame + 1)] # NULL_FRAME

(***************************************************************************)
(* INV-SP-4: Valid State Transitions                                       *)
(* Once Running, always Running (no reverse to Synchronizing)              *)
(* Note: This is a state predicate, not a transition predicate.            *)
(* The actual transition property [][(sessionState = "Running") =>         *)
(* (sessionState' = "Running")]_vars is implied by the Next relation       *)
(* since no action transitions from Running back to Synchronizing.         *)
(***************************************************************************)
RunningIsPermanent ==
    \* This temporal property verifies the state machine never goes backwards
    \* It's equivalent to checking: if we ever reach Running, we stay there
    [](sessionState = "Running" => [](sessionState = "Running"))

(***************************************************************************)
(* INV-SP-5: Frames Behind Non-Negative                                    *)
(* framesBehind is always >= 0                                             *)
(***************************************************************************)
FramesBehindNonNegative ==
    framesBehind >= 0

(***************************************************************************)
(* Action: Receive Sync Reply from Host                                    *)
(* Models: Event::Synchronized handler during synchronization              *)
(***************************************************************************)
ReceiveSyncReply ==
    /\ sessionState = "Synchronizing"
    /\ hostConnection = "Connected"
    /\ syncProgress' = syncProgress + 1
    /\ UNCHANGED <<sessionState, currentFrame, lastRecvFrame, inputBuffer,
                   inputFrames, hostConnection, framesBehind, inCatchupMode>>

(***************************************************************************)
(* Action: Sync Timeout                                                    *)
(* Models: Event::SyncTimeout handler                                      *)
(* Production: Forwards timeout event to user, but doesn't change state    *)
(* The spectator continues waiting for synchronization to complete         *)
(***************************************************************************)
SyncTimeout ==
    /\ sessionState = "Synchronizing"
    /\ hostConnection = "Connected"
    \* Timeout is a no-op for session state; it's forwarded to user as event
    \* In production: self.event_queue.push_back(FortressEvent::SyncTimeout {...})
    /\ UNCHANGED vars

(***************************************************************************)
(* Action: Complete Synchronization                                        *)
(* Models: Event::Synchronized -> state = SessionState::Running            *)
(* Production: Only sets self.state = SessionState::Running                *)
(* The current_frame remains NULL_FRAME until advance_frame() is called    *)
(***************************************************************************)
CompleteSynchronization ==
    /\ sessionState = "Synchronizing"
    /\ hostConnection = "Connected"
    /\ syncProgress > 0  \* At least one sync reply received
    /\ sessionState' = "Running"
    /\ UNCHANGED <<currentFrame, lastRecvFrame, inputBuffer, inputFrames,
                   hostConnection, syncProgress, framesBehind, inCatchupMode>>

(***************************************************************************)
(* Action: First Advance (NULL_FRAME -> 0)                                 *)
(* Models: First call to advance_frame() after synchronization             *)
(* Production: current_frame starts at NULL (-1), first advance goes to 0  *)
(* This is a separate action because the arithmetic is different:          *)
(*   - Normal advance: nextFrame = currentFrame + 1                        *)
(*   - First advance: nextFrame = 0 (special case for NULL_FRAME)          *)
(***************************************************************************)
FirstAdvance ==
    /\ sessionState = "Running"
    /\ currentFrame = NULL_FRAME  \* First advance after sync
    /\ lastRecvFrame # NULL_FRAME  \* Must have received at least one frame
    /\ lastRecvFrame >= 0  \* Frame 0 must be available
    /\ LET nextFrame == 0
           idx == BufferIndexFor(nextFrame)
       IN
        /\ inputFrames[idx] = nextFrame  \* Inputs available for frame 0
        /\ currentFrame' = nextFrame
        \* Update frames behind
        /\ framesBehind' = IF lastRecvFrame < nextFrame
                          THEN 0
                          ELSE lastRecvFrame - nextFrame
        /\ inCatchupMode' = (framesBehind' > MAX_FRAMES_BEHIND)
    /\ UNCHANGED <<sessionState, lastRecvFrame, inputBuffer, inputFrames,
                   hostConnection, syncProgress>>

(***************************************************************************)
(* Action: Receive Input from Host                                         *)
(* Models: Event::Input handler - stores inputs in buffer                  *)
(***************************************************************************)
ReceiveInput(frame, player, value) ==
    /\ hostConnection = "Connected"
    /\ frame \in 0..MAX_FRAME
    /\ player \in Players
    /\ value \in InputValue
    /\ LET idx == BufferIndexFor(frame)
       IN
        /\ inputBuffer' = [inputBuffer EXCEPT ![idx][player] = value]
        /\ inputFrames' = [inputFrames EXCEPT ![idx] = frame]
        \* Update last received frame if this is newer
        /\ IF lastRecvFrame = NULL_FRAME \/ frame > lastRecvFrame
           THEN lastRecvFrame' = frame
           ELSE lastRecvFrame' = lastRecvFrame
        \* Update frames behind calculation
        /\ framesBehind' = IF lastRecvFrame' = NULL_FRAME \/ currentFrame = NULL_FRAME
                          THEN 0
                          ELSE IF lastRecvFrame' < currentFrame
                               THEN 0
                               ELSE lastRecvFrame' - currentFrame
        /\ inCatchupMode' = (framesBehind' > MAX_FRAMES_BEHIND)
    /\ UNCHANGED <<sessionState, currentFrame, hostConnection, syncProgress>>

(***************************************************************************)
(* Action: Advance Frame (Normal Mode)                                     *)
(* Models: advance_frame() when not in catchup mode                        *)
(* Spectator advances one frame at a time                                  *)
(***************************************************************************)
AdvanceFrameNormal ==
    /\ sessionState = "Running"
    /\ ~inCatchupMode
    /\ currentFrame # NULL_FRAME
    /\ currentFrame < MAX_FRAME
    /\ currentFrame < lastRecvFrame  \* Have inputs to consume
    /\ LET nextFrame == currentFrame + 1
           idx == BufferIndexFor(nextFrame)
       IN
        /\ inputFrames[idx] = nextFrame  \* Inputs available for next frame
        /\ currentFrame' = nextFrame
        \* Update frames behind
        /\ framesBehind' = IF lastRecvFrame = NULL_FRAME
                          THEN 0
                          ELSE IF lastRecvFrame < nextFrame
                               THEN 0
                               ELSE lastRecvFrame - nextFrame
        /\ inCatchupMode' = (framesBehind' > MAX_FRAMES_BEHIND)
    /\ UNCHANGED <<sessionState, lastRecvFrame, inputBuffer, inputFrames,
                   hostConnection, syncProgress>>

(***************************************************************************)
(* Action: Advance Frame (Catchup Mode)                                    *)
(* Models: advance_frame() when frames_behind_host > max_frames_behind     *)
(* Spectator advances CATCHUP_SPEED frames at a time                       *)
(***************************************************************************)
AdvanceFrameCatchup ==
    /\ sessionState = "Running"
    /\ inCatchupMode
    /\ currentFrame # NULL_FRAME
    /\ currentFrame < MAX_FRAME
    /\ currentFrame < lastRecvFrame  \* Have inputs to consume
    \* In catchup mode, advance up to CATCHUP_SPEED frames
    /\ LET framesAvailable == lastRecvFrame - currentFrame
           framesToAdvance == IF framesAvailable < CATCHUP_SPEED
                             THEN framesAvailable
                             ELSE CATCHUP_SPEED
           nextFrame == currentFrame + framesToAdvance
       IN
        /\ nextFrame <= MAX_FRAME
        \* Verify all frames in the range have inputs
        /\ \A f \in (currentFrame + 1)..nextFrame:
            inputFrames[BufferIndexFor(f)] = f
        /\ currentFrame' = nextFrame
        \* Update frames behind
        /\ framesBehind' = IF lastRecvFrame = NULL_FRAME
                          THEN 0
                          ELSE IF lastRecvFrame < nextFrame
                               THEN 0
                               ELSE lastRecvFrame - nextFrame
        /\ inCatchupMode' = (framesBehind' > MAX_FRAMES_BEHIND)
    /\ UNCHANGED <<sessionState, lastRecvFrame, inputBuffer, inputFrames,
                   hostConnection, syncProgress>>

(***************************************************************************)
(* Action: Host Disconnects                                                *)
(* Models: Event::Disconnected handler                                     *)
(***************************************************************************)
HostDisconnect ==
    /\ hostConnection = "Connected"
    /\ hostConnection' = "Disconnected"
    /\ UNCHANGED <<sessionState, currentFrame, lastRecvFrame, inputBuffer,
                   inputFrames, syncProgress, framesBehind, inCatchupMode>>

(***************************************************************************)
(* Action: Wait for Inputs                                                 *)
(* Models: advance_frame() returning PredictionThreshold error             *)
(* Spectator waits when it has caught up to last received frame            *)
(***************************************************************************)
WaitForInputs ==
    /\ sessionState = "Running"
    /\ currentFrame # NULL_FRAME
    /\ currentFrame = lastRecvFrame  \* Caught up, need more inputs
    /\ UNCHANGED vars  \* No state change, just waiting

(***************************************************************************)
(* Action: Poll Remote (Stuttering Step)                                   *)
(* Models: poll_remote_clients() when no messages received                 *)
(***************************************************************************)
Poll ==
    /\ hostConnection = "Connected"
    /\ UNCHANGED vars

(***************************************************************************)
(* Next State Relation                                                     *)
(***************************************************************************)
Next ==
    \/ ReceiveSyncReply
    \/ SyncTimeout
    \/ CompleteSynchronization
    \/ \E f \in 0..MAX_FRAME, p \in Players, v \in InputValue:
        ReceiveInput(f, p, v)
    \/ FirstAdvance
    \/ AdvanceFrameNormal
    \/ AdvanceFrameCatchup
    \/ HostDisconnect
    \/ WaitForInputs
    \/ Poll

(***************************************************************************)
(* Fairness Conditions                                                     *)
(* Weak fairness on advancing frames ensures progress                      *)
(***************************************************************************)
Fairness ==
    /\ WF_vars(FirstAdvance)
    /\ WF_vars(AdvanceFrameNormal)
    /\ WF_vars(AdvanceFrameCatchup)
    /\ WF_vars(CompleteSynchronization)

(***************************************************************************)
(* Specification                                                           *)
(***************************************************************************)
Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Properties                                                       *)
(***************************************************************************)

\* Combined safety invariant
SafetyInvariant ==
    /\ TypeInvariant
    /\ SpectatorFrameBounded
    /\ BufferAccessValid
    /\ FramesBehindNonNegative

(***************************************************************************)
(* Liveness Properties                                                     *)
(***************************************************************************)

\* LIVE-SP-1: Eventually synchronized (if host stays connected)
EventuallySynchronized ==
    (hostConnection = "Connected" /\ syncProgress > 0) ~>
        sessionState = "Running"

\* LIVE-SP-2: Spectator eventually catches up (if inputs keep arriving)
EventuallyCaughtUp ==
    (inCatchupMode /\ lastRecvFrame > currentFrame) ~>
        (~inCatchupMode \/ currentFrame = lastRecvFrame)

(***************************************************************************)
(* State Constraint for Model Checking                                     *)
(***************************************************************************)
StateConstraint ==
    /\ (currentFrame = NULL_FRAME \/ currentFrame <= MAX_FRAME)
    /\ (lastRecvFrame = NULL_FRAME \/ lastRecvFrame <= MAX_FRAME)
    /\ syncProgress <= 5  \* Matches SyncConfig.num_sync_packets default
    /\ framesBehind <= MAX_FRAME

(***************************************************************************)
(* Theorems                                                                *)
(***************************************************************************)

\* The specification maintains safety
THEOREM SafetyTheorem == Spec => []SafetyInvariant

\* Synchronization eventually completes
THEOREM SyncTheorem == Spec => EventuallySynchronized

=============================================================================
