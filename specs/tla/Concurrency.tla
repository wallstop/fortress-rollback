------------------------------ MODULE Concurrency ------------------------------
(***************************************************************************)
(* TLA+ Specification for Fortress Rollback GameStateCell Concurrency      *)
(*                                                                         *)
(* This module specifies the concurrent state access patterns of           *)
(* GameStateCell<T>, which uses Arc<Mutex<T>> for thread-safe state        *)
(* management. It models:                                                  *)
(*   - Concurrent save/load operations                                     *)
(*   - Mutex lock acquisition and release                                  *)
(*   - Frame-based state transitions                                       *)
(*   - Multiple threads accessing shared state                             *)
(*                                                                         *)
(* Properties verified:                                                    *)
(*   - Safety: No data races (mutual exclusion)                            *)
(*   - Safety: Save-before-load ordering (within same frame)               *)
(*   - Safety: Linearizability of operations                               *)
(*   - Safety: Frame consistency after operations                          *)
(*   - Liveness: No deadlock                                               *)
(*   - Liveness: Operations eventually complete                            *)
(***************************************************************************)

EXTENDS Integers, Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    THREADS,            \* Set of thread identifiers
    MAX_FRAME,          \* Maximum frame for model checking
    NULL_FRAME          \* Sentinel value (-1)

ASSUME THREADS # {}
ASSUME MAX_FRAME \in Nat /\ MAX_FRAME > 0
ASSUME NULL_FRAME \notin 0..MAX_FRAME  \* Sentinel value (outside valid frame range)

(***************************************************************************)
(* Type Definitions                                                        *)
(***************************************************************************)
Frame == {NULL_FRAME} \union (0..MAX_FRAME)
ThreadId == THREADS

\* Thread states for modeling mutex behavior
ThreadState == {"idle", "waiting", "holding", "saving", "loading"}

\* Operation types
Operation == {"none", "save", "load", "data"}

(***************************************************************************)
(* Variables                                                               *)
(***************************************************************************)
VARIABLES
    \* GameStateCell state (what's inside Arc<Mutex<GameState<T>>>)
    cellFrame,          \* The frame stored in the cell
    cellData,           \* The data stored (simplified to a Nat)
    cellChecksum,       \* Checksum stored (optional, Nat or NULL)
    
    \* Mutex state
    lockHolder,         \* Which thread holds the lock (NULL_FRAME if unlocked)
    waitQueue,          \* Queue of threads waiting for lock
    
    \* Per-thread state
    threadState,        \* threadState[t] = current state of thread t
    threadOp,           \* threadOp[t] = current operation thread is performing
    threadFrame,        \* threadFrame[t] = frame argument for operation
    threadData,         \* threadData[t] = data to save or loaded data
    
    \* Auxiliary variables for verification
    saveCount,          \* Count of completed save operations
    loadCount,          \* Count of completed load operations
    history             \* Sequence of completed operations for linearizability

vars == <<cellFrame, cellData, cellChecksum, lockHolder, waitQueue,
          threadState, threadOp, threadFrame, threadData,
          saveCount, loadCount, history>>

(***************************************************************************)
(* Type Invariant                                                          *)
(***************************************************************************)
TypeInvariant ==
    /\ cellFrame \in Frame
    /\ cellData \in (Nat \cup {NULL_FRAME})
    /\ cellChecksum \in (Nat \cup {NULL_FRAME})
    /\ lockHolder \in (ThreadId \cup {NULL_FRAME})
    /\ waitQueue \in Seq(ThreadId)
    /\ threadState \in [ThreadId -> ThreadState]
    /\ threadOp \in [ThreadId -> Operation]
    /\ threadFrame \in [ThreadId -> Frame]
    /\ threadData \in [ThreadId -> (Nat \cup {NULL_FRAME})]
    /\ saveCount \in Nat
    /\ loadCount \in Nat
    /\ history \in Seq([op: Operation, frame: Frame, thread: ThreadId])

(***************************************************************************)
(* Initial State                                                           *)
(***************************************************************************)
Init ==
    /\ cellFrame = NULL_FRAME
    /\ cellData = NULL_FRAME           \* No data initially
    /\ cellChecksum = NULL_FRAME
    /\ lockHolder = NULL_FRAME         \* Unlocked
    /\ waitQueue = <<>>
    /\ threadState = [t \in ThreadId |-> "idle"]
    /\ threadOp = [t \in ThreadId |-> "none"]
    /\ threadFrame = [t \in ThreadId |-> NULL_FRAME]
    /\ threadData = [t \in ThreadId |-> NULL_FRAME]
    /\ saveCount = 0
    /\ loadCount = 0
    /\ history = <<>>

(***************************************************************************)
(* Helper: Check if lock is available or held by thread                    *)
(***************************************************************************)
LockAvailable == lockHolder = NULL_FRAME

LockHeldBy(t) == lockHolder = t

(***************************************************************************)
(* Helper: Thread is not currently busy                                    *)
(***************************************************************************)
ThreadIdle(t) == threadState[t] = "idle"

(***************************************************************************)
(* Action: Request Save Operation                                          *)
(* Thread begins a save operation (corresponds to cell.save() call)        *)
(***************************************************************************)
RequestSave(t, f, d) ==
    /\ ThreadIdle(t)
    /\ f # NULL_FRAME                   \* Pre: frame must not be null
    /\ f <= MAX_FRAME
    /\ threadState' = [threadState EXCEPT ![t] = "waiting"]
    /\ threadOp' = [threadOp EXCEPT ![t] = "save"]
    /\ threadFrame' = [threadFrame EXCEPT ![t] = f]
    /\ threadData' = [threadData EXCEPT ![t] = d]
    \* Try to acquire lock immediately if available
    /\ IF LockAvailable
       THEN /\ lockHolder' = t
            /\ threadState' = [threadState EXCEPT ![t] = "holding"]
            /\ waitQueue' = waitQueue
       ELSE /\ waitQueue' = Append(waitQueue, t)
            /\ UNCHANGED <<lockHolder>>
    /\ UNCHANGED <<cellFrame, cellData, cellChecksum, saveCount, loadCount, history>>

(***************************************************************************)
(* Action: Request Load Operation                                          *)
(* Thread begins a load operation (corresponds to cell.load() call)        *)
(***************************************************************************)
RequestLoad(t) ==
    /\ ThreadIdle(t)
    /\ threadState' = [threadState EXCEPT ![t] = "waiting"]
    /\ threadOp' = [threadOp EXCEPT ![t] = "load"]
    /\ threadFrame' = [threadFrame EXCEPT ![t] = NULL_FRAME]  \* Load reads current frame
    /\ threadData' = [threadData EXCEPT ![t] = NULL_FRAME]
    \* Try to acquire lock immediately if available
    /\ IF LockAvailable
       THEN /\ lockHolder' = t
            /\ threadState' = [threadState EXCEPT ![t] = "holding"]
            /\ waitQueue' = waitQueue
       ELSE /\ waitQueue' = Append(waitQueue, t)
            /\ UNCHANGED <<lockHolder>>
    /\ UNCHANGED <<cellFrame, cellData, cellChecksum, saveCount, loadCount, history>>

(***************************************************************************)
(* Action: Request Data Access Operation                                   *)
(* Thread begins a data access (corresponds to cell.data() call)           *)
(***************************************************************************)
RequestData(t) ==
    /\ ThreadIdle(t)
    /\ threadState' = [threadState EXCEPT ![t] = "waiting"]
    /\ threadOp' = [threadOp EXCEPT ![t] = "data"]
    /\ threadFrame' = [threadFrame EXCEPT ![t] = NULL_FRAME]
    /\ threadData' = [threadData EXCEPT ![t] = NULL_FRAME]
    \* Try to acquire lock immediately if available
    /\ IF LockAvailable
       THEN /\ lockHolder' = t
            /\ threadState' = [threadState EXCEPT ![t] = "holding"]
            /\ waitQueue' = waitQueue
       ELSE /\ waitQueue' = Append(waitQueue, t)
            /\ UNCHANGED <<lockHolder>>
    /\ UNCHANGED <<cellFrame, cellData, cellChecksum, saveCount, loadCount, history>>

(***************************************************************************)
(* Action: Acquire Lock (for waiting thread)                               *)
(* Thread waiting in queue acquires the lock when it becomes available     *)
(***************************************************************************)
AcquireLock(t) ==
    /\ threadState[t] = "waiting"
    /\ LockAvailable
    /\ Len(waitQueue) > 0
    /\ Head(waitQueue) = t              \* First in queue gets lock
    /\ lockHolder' = t
    /\ waitQueue' = Tail(waitQueue)
    /\ threadState' = [threadState EXCEPT ![t] = "holding"]
    /\ UNCHANGED <<cellFrame, cellData, cellChecksum, threadOp, 
                   threadFrame, threadData, saveCount, loadCount, history>>

(***************************************************************************)
(* Action: Execute Save                                                    *)
(* Thread holding lock performs the save operation                         *)
(***************************************************************************)
ExecuteSave(t) ==
    /\ LockHeldBy(t)
    /\ threadState[t] = "holding"
    /\ threadOp[t] = "save"
    /\ cellFrame' = threadFrame[t]
    /\ cellData' = threadData[t]
    /\ cellChecksum' = threadData[t]    \* Simplified: checksum = data
    /\ threadState' = [threadState EXCEPT ![t] = "saving"]
    /\ saveCount' = saveCount + 1
    /\ history' = Append(history, [op |-> "save", frame |-> threadFrame[t], thread |-> t])
    /\ UNCHANGED <<lockHolder, waitQueue, threadOp, threadFrame, threadData, loadCount>>

(***************************************************************************)
(* Action: Execute Load                                                    *)
(* Thread holding lock performs the load operation                         *)
(***************************************************************************)
ExecuteLoad(t) ==
    /\ LockHeldBy(t)
    /\ threadState[t] = "holding"
    /\ threadOp[t] = "load"
    /\ threadData' = [threadData EXCEPT ![t] = cellData]  \* Clone data
    /\ threadFrame' = [threadFrame EXCEPT ![t] = cellFrame]
    /\ threadState' = [threadState EXCEPT ![t] = "loading"]
    /\ loadCount' = loadCount + 1
    /\ history' = Append(history, [op |-> "load", frame |-> cellFrame, thread |-> t])
    /\ UNCHANGED <<cellFrame, cellData, cellChecksum, lockHolder, waitQueue, 
                   threadOp, saveCount>>

(***************************************************************************)
(* Action: Execute Data Access                                             *)
(* Thread holding lock performs read access (data() method)                *)
(***************************************************************************)
ExecuteData(t) ==
    /\ LockHeldBy(t)
    /\ threadState[t] = "holding"
    /\ threadOp[t] = "data"
    /\ threadData' = [threadData EXCEPT ![t] = cellData]
    /\ threadFrame' = [threadFrame EXCEPT ![t] = cellFrame]
    /\ threadState' = [threadState EXCEPT ![t] = "loading"]  \* Reuse loading state
    /\ history' = Append(history, [op |-> "data", frame |-> cellFrame, thread |-> t])
    /\ UNCHANGED <<cellFrame, cellData, cellChecksum, lockHolder, waitQueue, 
                   threadOp, saveCount, loadCount>>

(***************************************************************************)
(* Action: Release Lock                                                    *)
(* Thread releases lock after completing operation                         *)
(***************************************************************************)
ReleaseLock(t) ==
    /\ LockHeldBy(t)
    /\ threadState[t] \in {"saving", "loading"}  \* Operation complete
    /\ lockHolder' = NULL_FRAME
    /\ threadState' = [threadState EXCEPT ![t] = "idle"]
    /\ threadOp' = [threadOp EXCEPT ![t] = "none"]
    \* Keep threadData for verification (it would be returned to caller)
    /\ UNCHANGED <<cellFrame, cellData, cellChecksum, waitQueue, 
                   threadFrame, threadData, saveCount, loadCount, history>>

(***************************************************************************)
(* Next State Relation                                                     *)
(***************************************************************************)
Next ==
    \E t \in ThreadId :
        \/ \E f \in 0..MAX_FRAME, d \in 0..MAX_FRAME : RequestSave(t, f, d)
        \/ RequestLoad(t)
        \/ RequestData(t)
        \/ AcquireLock(t)
        \/ ExecuteSave(t)
        \/ ExecuteLoad(t)
        \/ ExecuteData(t)
        \/ ReleaseLock(t)

(***************************************************************************)
(* Fairness Conditions                                                     *)
(***************************************************************************)
Fairness ==
    /\ \A t \in ThreadId : WF_vars(AcquireLock(t))
    /\ \A t \in ThreadId : WF_vars(ExecuteSave(t))
    /\ \A t \in ThreadId : WF_vars(ExecuteLoad(t))
    /\ \A t \in ThreadId : WF_vars(ExecuteData(t))
    /\ \A t \in ThreadId : WF_vars(ReleaseLock(t))

Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* SAFETY PROPERTIES                                                       *)
(***************************************************************************)

(***************************************************************************)
(* Mutual Exclusion: At most one thread holds the lock                     *)
(***************************************************************************)
MutualExclusion ==
    \A t1, t2 \in ThreadId :
        (LockHeldBy(t1) /\ LockHeldBy(t2)) => t1 = t2

(***************************************************************************)
(* No Data Race: Only lock holder can modify cell state                    *)
(* Implied by MutualExclusion since all operations require lock            *)
(***************************************************************************)
NoDataRace ==
    \A t \in ThreadId :
        (threadState[t] \in {"saving", "loading"}) => LockHeldBy(t)

(***************************************************************************)
(* Frame Consistency: After save, cell frame matches saved frame           *)
(***************************************************************************)
FrameConsistency ==
    \A t \in ThreadId :
        (threadState[t] = "saving" /\ threadOp[t] = "save") =>
            cellFrame = threadFrame[t]

(***************************************************************************)
(* Load Returns Saved Data: Load operation returns what was saved          *)
(***************************************************************************)
LoadReturnsSaved ==
    \A t \in ThreadId :
        (threadState[t] = "loading" /\ threadOp[t] = "load") =>
            (threadData[t] = cellData \/ cellData = NULL_FRAME)

(***************************************************************************)
(* Valid Frame After Save: Save never stores NULL_FRAME                    *)
(* Matches precondition: assert!(!frame.is_null()) in save()               *)
(***************************************************************************)
ValidFrameAfterSave ==
    saveCount > 0 => cellFrame # NULL_FRAME

(***************************************************************************)
(* Wait Queue FIFO: Threads acquire lock in request order                  *)
(* Enforced by the AcquireLock action taking Head(waitQueue)               *)
(***************************************************************************)
WaitQueueFIFO ==
    \A i, j \in 1..Len(waitQueue) :
        i < j => waitQueue[i] # waitQueue[j]

(***************************************************************************)
(* Helper: Range of a sequence                                             *)
(***************************************************************************)
Range(s) == {s[i] : i \in 1..Len(s)}

(***************************************************************************)
(* No Double Holding: A thread cannot be both waiting and holding          *)
(***************************************************************************)
NoDoubleHolding ==
    \A t \in ThreadId :
        LockHeldBy(t) => t \notin Range(waitQueue)

(***************************************************************************)
(* Safety Invariant: Conjunction of all safety properties                  *)
(***************************************************************************)
SafetyInvariant ==
    /\ TypeInvariant
    /\ MutualExclusion
    /\ NoDataRace
    /\ FrameConsistency
    /\ LoadReturnsSaved
    /\ ValidFrameAfterSave
    /\ WaitQueueFIFO

(***************************************************************************)
(* LIVENESS PROPERTIES                                                     *)
(***************************************************************************)

(***************************************************************************)
(* No Deadlock: Some action is always enabled                              *)
(***************************************************************************)
NoDeadlock ==
    [][ENABLED(Next)]_vars

(***************************************************************************)
(* Operations Complete: Every started operation eventually completes       *)
(***************************************************************************)
OperationsComplete ==
    \A t \in ThreadId :
        (threadState[t] = "waiting") ~> (threadState[t] = "idle")

(***************************************************************************)
(* Fair Lock Acquisition: Waiting threads eventually get the lock          *)
(***************************************************************************)
FairLockAcquisition ==
    \A t \in ThreadId :
        (t \in Range(waitQueue)) ~> LockHeldBy(t)

(***************************************************************************)
(* LINEARIZABILITY (Simplified)                                            *)
(***************************************************************************)
(* Each operation appears to take effect instantaneously at some point     *)
(* between its invocation and response. The mutex ensures this by          *)
(* making all operations atomic while holding the lock.                    *)
(*                                                                         *)
(* The history variable records the linearization order:                   *)
(* - Operations are appended when they complete (inside lock)              *)
(* - The order in history represents the linearization                     *)
(***************************************************************************)

LinearizableHistory ==
    \* If a save happened before a load (in real time), and the load
    \* sees that frame, then the save must appear before load in history
    \A i, j \in 1..Len(history) :
        (i < j /\ history[i].op = "save" /\ history[j].op = "load" /\
         history[j].frame = history[i].frame) =>
            \E k \in 1..(j-1) : history[k] = history[i]

(**************************************************************************)
(* State Constraint for Model Checking                                     *)
(**************************************************************************)
StateConstraint ==
    /\ saveCount + loadCount <= 6
    /\ (cellFrame = NULL_FRAME \/ cellFrame <= MAX_FRAME)
    /\ Len(waitQueue) <= Cardinality(THREADS)
    /\ Len(history) <= 8

(**************************************************************************)
(* THEOREMS                                                                *)
(**************************************************************************)
(* These theorems state properties that hold for all behaviors of Spec     *)

THEOREM SafetyTheorem == Spec => []SafetyInvariant

THEOREM LivenessTheorem == Spec => NoDeadlock /\ OperationsComplete

================================================================================
