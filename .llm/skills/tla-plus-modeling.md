# TLA+ Modeling — Formal Specification for Distributed Systems

> **This document provides guidance for writing TLA+ specifications and adapting production code to formal models.**
> TLA+ is used in Fortress Rollback to verify correctness of the rollback mechanism, network protocol, and concurrency primitives.

## Core Philosophy: Specs Are Not Code

The fundamental insight: **TLA+ specifications are abstract models of behavior, NOT direct translations of implementation code.**

```tla
(* ❌ WRONG: Trying to model implementation details *)
ProcessPacket ==
    /\ packet.type = "DATA"
    /\ packet.checksum = ComputeChecksum(packet.payload)
    /\ buffer := Append(buffer, DecodePayload(packet.payload))
    /\ ...dozens more implementation details...

(* ✅ RIGHT: Model essential behavior *)
ReceiveInput ==
    /\ \E input \in PendingInputs:
        /\ inputs' = [inputs EXCEPT ![input.player][input.frame] = input.value]
        /\ confirmed' = [confirmed EXCEPT ![input.player][input.frame] = TRUE]
```

### Key Principles

1. **Model what could go wrong subtly** — Focus on concurrency, coordination, and protocol logic
2. **Skip obvious implementation details** — Data transformations, byte-level parsing, performance code
3. **Embrace abstraction** — A 100-line spec can verify properties of 10,000 lines of code
4. **Use specs for design validation** — Find bugs before writing code, not after

---

## TLA+ Specification Structure

### Minimal Specification Template

Every model-checkable TLA+ spec needs these components:

```tla
---- MODULE ModuleName ----
EXTENDS Integers, Sequences, FiniteSets

CONSTANTS
    MAX_VALUE,      \* Bounds for model checking
    NUM_PROCESSES   \* Number of concurrent entities

ASSUME MAX_VALUE \in Nat /\ MAX_VALUE > 0
ASSUME NUM_PROCESSES \in Nat /\ NUM_PROCESSES > 0

(***************************************************************************)
(* Type Definitions                                                        *)
(***************************************************************************)
ValidValues == 0..MAX_VALUE
Processes == 1..NUM_PROCESSES

(***************************************************************************)
(* Variables                                                               *)
(***************************************************************************)
VARIABLES
    state,          \* Primary system state
    pc              \* Program counter (for PlusCal or explicit state machines)

vars == <<state, pc>>

(***************************************************************************)
(* Type Invariant - Documents expected variable types                      *)
(***************************************************************************)
TypeInvariant ==
    /\ state \in [Processes -> ValidValues]
    /\ pc \in [Processes -> {"init", "running", "done"}]

(***************************************************************************)
(* Initial State                                                           *)
(***************************************************************************)
Init ==
    /\ state = [p \in Processes |-> 0]
    /\ pc = [p \in Processes |-> "init"]

(***************************************************************************)
(* Actions - State Transitions                                             *)
(***************************************************************************)
Start(p) ==
    /\ pc[p] = "init"
    /\ pc' = [pc EXCEPT ![p] = "running"]
    /\ UNCHANGED state

DoWork(p) ==
    /\ pc[p] = "running"
    /\ state[p] < MAX_VALUE
    /\ state' = [state EXCEPT ![p] = state[p] + 1]
    /\ UNCHANGED pc

Finish(p) ==
    /\ pc[p] = "running"
    /\ pc' = [pc EXCEPT ![p] = "done"]
    /\ UNCHANGED state

(***************************************************************************)
(* Next-State Relation                                                     *)
(***************************************************************************)
Next ==
    \E p \in Processes:
        \/ Start(p)
        \/ DoWork(p)
        \/ Finish(p)

(***************************************************************************)
(* Fairness (for liveness properties)                                      *)
(***************************************************************************)
Fairness == \A p \in Processes: WF_vars(DoWork(p))

(***************************************************************************)
(* Specification                                                           *)
(***************************************************************************)
Spec == Init /\ [][Next]_vars /\ Fairness

(***************************************************************************)
(* Safety Invariants                                                       *)
(***************************************************************************)
SafetyInvariant ==
    \A p \in Processes: state[p] <= MAX_VALUE

(***************************************************************************)
(* Liveness Properties                                                     *)
(***************************************************************************)
EventuallyDone == <>(\A p \in Processes: pc[p] = "done")

====
```

---

## What to Model vs What to Skip

### Model These Elements

| What to Model | Why | Example |
|--------------|-----|---------|
| **State machine transitions** | Core protocol correctness | `Initializing → Synchronizing → Running` |
| **Concurrency and interleavings** | Race conditions, deadlocks | Multiple players sending inputs |
| **Coordination logic** | Synchronization bugs | Frame advancement, rollback triggers |
| **Message ordering** | Protocol violations | Input delivery order, acknowledgments |
| **Failure scenarios** | Resilience properties | Network drops, disconnects |
| **Invariants** | Safety properties | "Rollback depth ≤ MAX_PREDICTION" |

### Skip These Elements

| What to Skip | Why | Alternative |
|-------------|-----|-------------|
| **Byte-level serialization** | Implementation detail | Model as abstract messages |
| **Checksum algorithms** | Pure computation | Assume correct or use Boolean |
| **Performance optimizations** | Doesn't affect correctness | Model simple version |
| **Memory management** | Low-level detail | Assume unbounded or use bounds |
| **Error message formatting** | UI concern | Model as error states |
| **Logging/telemetry** | Side effects | Ignore entirely |

---

## Abstraction Levels

Choose the right abstraction level for your verification goals:

```
Design Level     →  "All peers eventually synchronize"
    ↓
Protocol Level   →  "Sync packets establish shared frame reference"
    ↓
Algorithm Level  →  "Input queue maintains FIFO, bounded depth"
    ↓
Code Level       →  Actual Rust implementation
```

### Guidelines

- **Start high, add detail only when needed** — Most bugs appear at higher abstraction levels
- **If model checking is slow, abstract more** — State space explosion means too much detail
- **Verify properties at multiple levels** — Refinement can prove levels consistent

---

## Common Modeling Patterns

### Pattern 1: State Machines

Model protocol states as explicit state variables:

```tla
(* State machine pattern - used in NetworkProtocol.tla *)
States == {"Initializing", "Synchronizing", "Running", "Disconnected"}

Init == state = "Initializing"

StartSync ==
    /\ state = "Initializing"
    /\ state' = "Synchronizing"

BecomeReady ==
    /\ state = "Synchronizing"
    /\ syncComplete
    /\ state' = "Running"

(* Validate only legal transitions *)
ValidTransitions == {
    <<"Initializing", "Synchronizing">>,
    <<"Synchronizing", "Running">>,
    <<"Running", "Disconnected">>
}

TransitionSafety == [][<<state, state'>> \in ValidTransitions]_state
```

### Pattern 2: Circular Buffers / Queues

Model bounded queues with wrap-around:

```tla
(* Input queue pattern - used in InputQueue.tla *)
CONSTANTS QUEUE_LENGTH, NULL_FRAME

VARIABLES head, tail, buffer

QueueLen == (tail - head + QUEUE_LENGTH) % QUEUE_LENGTH

Enqueue(value) ==
    /\ QueueLen < QUEUE_LENGTH - 1  \* Not full
    /\ buffer' = [buffer EXCEPT ![tail] = value]
    /\ tail' = (tail + 1) % QUEUE_LENGTH
    /\ UNCHANGED head

Dequeue ==
    /\ head /= tail  \* Not empty
    /\ head' = (head + 1) % QUEUE_LENGTH
    /\ UNCHANGED <<tail, buffer>>

(* Invariant: queue length always bounded *)
QueueBounded == QueueLen <= QUEUE_LENGTH
```

### Pattern 3: Nondeterministic Failures

Model network unreliability with `either`/`or`:

```tla
(* Network with message loss/reorder *)
SendMessage(msg) ==
    \/ \* Success: message delivered
       /\ network' = network \union {msg}
    \/ \* Failure: message lost
       /\ UNCHANGED network
    \/ \* Duplication: message delivered twice (at-least-once)
       /\ network' = network \union {msg, msg}

ReceiveMessage ==
    /\ \E msg \in network:
        /\ ProcessMessage(msg)
        /\ network' = network \ {msg}
```

### Pattern 4: Rollback and State Restoration

Model save/load patterns for rollback systems:

```tla
(* Rollback pattern - used in Rollback.tla *)
VARIABLES currentFrame, savedStates, inRollback, rollbackTarget

SaveState ==
    /\ ~inRollback
    /\ savedStates' = [savedStates EXCEPT ![currentFrame] = CurrentState]
    /\ UNCHANGED <<currentFrame, inRollback, rollbackTarget>>

StartRollback(targetFrame) ==
    /\ targetFrame < currentFrame
    /\ targetFrame >= currentFrame - MAX_PREDICTION
    /\ savedStates[targetFrame] /= <<>>  \* State exists
    /\ inRollback' = TRUE
    /\ rollbackTarget' = targetFrame
    /\ UNCHANGED <<currentFrame, savedStates>>

LoadState ==
    /\ inRollback
    /\ currentFrame' = rollbackTarget
    /\ \* Restore from savedStates[rollbackTarget]
    /\ UNCHANGED <<savedStates, rollbackTarget>>

FinishRollback ==
    /\ inRollback
    /\ currentFrame = rollbackTarget
    /\ inRollback' = FALSE
    /\ UNCHANGED <<currentFrame, savedStates, rollbackTarget>>
```

### Pattern 5: Multi-Process Coordination

Model concurrent entities with existential quantification:

```tla
(* Multi-player input processing *)
CONSTANTS Players
VARIABLES inputs, confirmed

(* Any player can send input for any frame *)
SendInput(player, frame, value) ==
    /\ frame >= MinFrame
    /\ frame <= MaxFrame
    /\ inputs' = [inputs EXCEPT ![player][frame] = value]
    /\ UNCHANGED confirmed

(* Inputs become confirmed when received from network *)
ConfirmInput(player, frame) ==
    /\ inputs[player][frame] /= NULL
    /\ confirmed' = [confirmed EXCEPT ![player][frame] = TRUE]
    /\ UNCHANGED inputs

Next ==
    \E p \in Players, f \in Frames, v \in InputValues:
        \/ SendInput(p, f, v)
        \/ ConfirmInput(p, f)
```

---

## Helper Operators — Write More Than You Think

> "Most TLA+ specs don't have enough helpers." — Hillel Wayne

```tla
(***************************************************************************)
(* Useful Helper Operators                                                 *)
(***************************************************************************)

\* Set addition shorthand
set ++ elem == set \union {elem}

\* Sequence append shorthand
seq (+) elem == Append(seq, elem)

\* Minimum/Maximum
Min(a, b) == IF a < b THEN a ELSE b
Max(a, b) == IF a > b THEN a ELSE b

\* Clamp to range
Clamp(val, lo, hi) == Min(Max(val, lo), hi)

\* Check if sequence contains element
Contains(seq, elem) == \E i \in 1..Len(seq): seq[i] = elem

\* Range of a sequence (set of all elements)
Range(seq) == {seq[i]: i \in 1..Len(seq)}

\* All sequences of elements from set, up to length n
SeqOf(set, n) == UNION {[1..m -> set] : m \in 0..n}

\* Minimum element of non-empty set
SetMin(S) == CHOOSE x \in S: \A y \in S: x <= y

\* Maximum confirmed frame across all players
MaxConfirmedFrame(players, confirmed) ==
    LET frames == {f \in Frames: \A p \in players: confirmed[p][f]}
    IN IF frames = {} THEN NULL_FRAME ELSE SetMin(frames)
```

---

## Safety vs Liveness Properties

### Safety Properties

"Nothing bad ever happens" — checked at every state:

```tla
(* Safety: Rollback depth bounded *)
RollbackBounded ==
    inRollback => (currentFrame - rollbackTarget <= MAX_PREDICTION)

(* Safety: Queue never overflows *)
QueueSafe == QueueLen <= QUEUE_LENGTH

(* Safety: Type invariant always holds *)
TypeOK ==
    /\ currentFrame \in 0..MAX_FRAME
    /\ savedStates \in [0..MAX_FRAME -> States \union {<<>>}]
```

### Liveness Properties

"Something good eventually happens" — requires fairness:

```tla
(* Liveness: Eventually synchronized *)
EventuallySynced == <>(state = "Running")

(* Liveness: Rollback completes *)
RollbackCompletes == inRollback ~> ~inRollback

(* Fairness: Required for liveness *)
Fairness ==
    /\ WF_vars(Next)  \* Weak fairness on all actions
    /\ SF_vars(FinishRollback)  \* Strong fairness on completion
```

**Important**: Liveness checking is much slower than safety checking. Run safety checks first, then add liveness for critical properties.

---

## PlusCal vs Pure TLA+

### Use PlusCal When

- Learning TLA+ (more familiar imperative syntax)
- Modeling sequential algorithms
- Quick prototyping of concurrent processes
- The algorithm has clear "steps"

```pluscal
(*--algorithm rollback
variables
    frame = 0;
    saved = [f \in 0..MAX_FRAME |-> NULL];

begin
    MainLoop:
    while frame < MAX_FRAME do
        SaveState:
            saved[frame] := GetState();
        Advance:
            frame := frame + 1;
    end while;
end algorithm; *)
```

### Use Pure TLA+ When

- Complex action composition needed
- Subtle fairness requirements
- Writing reusable, composable modules
- Need full control over state transitions

```tla
Advance ==
    /\ frame < MAX_FRAME
    /\ saved' = [saved EXCEPT ![frame] = GetState()]
    /\ frame' = frame + 1
```

### Key PlusCal Rules

1. **Labels = atomic units** — Everything in a label happens instantaneously
2. **One assignment per variable per label** — Use `||` for multiple parts
3. **`await` blocks** — Process waits until condition is true
4. **`either/or` for nondeterminism** — Model failures, choices

---

## Model Checking Optimization

### State Space Management

```tla
(* Use small constants for model checking *)
CONSTANTS
    MAX_FRAME = 5,      \* Small for tractability
    MAX_PREDICTION = 2,
    NUM_PLAYERS = 2

(* Add state constraints to limit exploration *)
CONSTRAINT
    currentFrame <= MAX_FRAME
    /\ QueueLen <= QUEUE_LENGTH

(* Use symmetry for interchangeable values *)
SYMMETRY Players  \* Reduces states by n!
```

### Performance Tips

1. **Start with tiny bounds** — Most bugs appear with small state spaces
2. **Separate safety and liveness** — Run safety first (faster)
3. **Use action constraints** — Limit which actions TLC explores
4. **Increase workers** — Use `-workers auto` or `-workers N`
5. **Enable checkpointing** — For long runs: `-checkpoint 60`

### Common TLC Options

```bash
# Basic model check
java -jar tla2tools.jar -config Spec.cfg Spec.tla

# Parallel execution
java -jar tla2tools.jar -workers auto -config Spec.cfg Spec.tla

# Memory settings for large models
java -Xmx8G -jar tla2tools.jar -workers 8 Spec.tla

# Generate trace on error
java -jar tla2tools.jar -dump dot,colorize states.dot Spec.tla
```

---

## Mapping Code Constructs to TLA+

### Rust to TLA+ Translation Guide

| Rust Construct | TLA+ Equivalent |
|----------------|-----------------|
| `enum State { A, B, C }` | `States == {"A", "B", "C"}` |
| `struct { field: T }` | `[field: T]` (record) |
| `Vec<T>` | `Seq(T)` or `[1..n -> T]` |
| `HashMap<K, V>` | `[K -> V]` (function) |
| `Option<T>` | `T \union {NULL}` |
| `Result<T, E>` | Model success/error states explicitly |
| `match state { ... }` | `CASE` or nested `IF/THEN/ELSE` |
| `if cond { ... }` | `/\ cond /\ ...` (as enabling condition) |
| `loop { ... }` | Recursive action or `while` in PlusCal |
| `thread::spawn` | Process in PlusCal or parallel composition |
| `Mutex<T>` | Lock variable + mutual exclusion invariant |
| `AtomicU32` | Variable with atomic read-modify-write actions |

### Example: Translating a Rust Enum

```rust
// Rust
enum ConnectionState {
    Initializing,
    Synchronizing { sync_remaining: u8 },
    Running,
    Disconnected,
}
```

```tla
(* TLA+ *)
ConnectionState ==
    [type: {"Initializing"}]
    \union [type: {"Synchronizing"}, sync_remaining: 0..255]
    \union [type: {"Running"}]
    \union [type: {"Disconnected"}]

\* Or simpler if sync_remaining not needed:
SimpleState == {"Initializing", "Synchronizing", "Running", "Disconnected"}
```

---

## Invariant Design Patterns

### Type Invariants (Always Include)

```tla
TypeInvariant ==
    /\ currentFrame \in 0..MAX_FRAME \union {NULL_FRAME}
    /\ playerInputs \in [Players -> [Frames -> InputValues \union {NULL}]]
    /\ inRollback \in BOOLEAN
```

### Range Invariants

```tla
\* Frame relationships
FrameRanges ==
    /\ lastConfirmedFrame <= currentFrame
    /\ lastSavedFrame <= currentFrame
    /\ rollbackTarget <= currentFrame - 1
```

### State Consistency Invariants

```tla
\* Saved state exists for rollback target
RollbackStateExists ==
    inRollback => savedStates[rollbackTarget] /= <<>>

\* No gaps in confirmed inputs
NoConfirmationGaps ==
    \A p \in Players, f \in 0..lastConfirmedFrame:
        confirmed[p][f] = TRUE
```

### Cross-Variable Invariants

```tla
\* Queue head/tail consistency
QueueConsistent ==
    /\ head \in 0..(QUEUE_LENGTH - 1)
    /\ tail \in 0..(QUEUE_LENGTH - 1)
    /\ (head = tail) <=> (QueueLen = 0)
```

---

## Common Pitfalls to Avoid

### Pitfall 1: Modeling Too Much Detail

```tla
(* ❌ Too detailed — models byte-level protocol *)
ProcessPacket ==
    /\ packet.header.version = 1
    /\ packet.header.type \in {0x01, 0x02, 0x03}
    /\ ValidChecksum(packet.payload, packet.checksum)
    /\ ...20 more conditions...

(* ✅ Right level — models essential behavior *)
ReceiveInput(player, frame, value) ==
    /\ inputs' = [inputs EXCEPT ![player][frame] = value]
```

### Pitfall 2: Forgetting UNCHANGED

```tla
(* ❌ WRONG — what happens to other variables? *)
Advance == frame' = frame + 1

(* ✅ RIGHT — explicitly state unchanged vars *)
Advance ==
    /\ frame' = frame + 1
    /\ UNCHANGED <<savedStates, inputs, confirmed>>
```

### Pitfall 3: Assuming Atomicity

```tla
(* ❌ WRONG — assumes read-modify-write is atomic *)
IncrementCounter ==
    counter' = counter + 1

(* ✅ RIGHT — model actual non-atomicity *)
ReadCounter(t) ==
    /\ local[t]' = counter
    /\ UNCHANGED counter

WriteCounter(t) ==
    /\ counter' = local[t] + 1
    /\ UNCHANGED local
```

### Pitfall 4: Wrong Quantifier with Implication

```tla
(* ❌ WRONG — always true (vacuously) *)
NoDuplicates == \E i, j \in 1..Len(seq): i /= j => seq[i] /= seq[j]

(* ✅ RIGHT — use /\ with \E *)
HasDuplicate == \E i, j \in 1..Len(seq): i /= j /\ seq[i] = seq[j]

(* ✅ RIGHT — use => with \A *)
NoDuplicates == \A i, j \in 1..Len(seq): i /= j => seq[i] /= seq[j]
```

### Pitfall 5: Unbounded State Space

```tla
(* ❌ WRONG — infinite states *)
VARIABLES counter  \* No upper bound!
Increment == counter' = counter + 1

(* ✅ RIGHT — bounded for model checking *)
CONSTANTS MAX_COUNTER
ASSUME MAX_COUNTER \in Nat
Increment ==
    /\ counter < MAX_COUNTER
    /\ counter' = counter + 1
```

---

## Integration with CI/CD

### Project Structure

```
specs/
├── tla/
│   ├── README.md           # Documentation
│   ├── Rollback.tla        # Spec
│   ├── Rollback.cfg        # Model config
│   ├── InputQueue.tla
│   ├── InputQueue.cfg
│   └── ...
scripts/
└── verify-tla.sh           # CI script
```

### CI Configuration (GitHub Actions)

```yaml
tla-verification:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/setup-java@v4
      with:
        distribution: 'temurin'
        java-version: '17'
    - name: Download TLA+ Tools
      run: |
        wget -q https://github.com/tlaplus/tlaplus/releases/download/v1.8.0/tla2tools.jar
    - name: Verify Specifications
      run: |
        for cfg in specs/tla/*.cfg; do
          spec="${cfg%.cfg}.tla"
          echo "Checking $spec..."
          java -jar tla2tools.jar -config "$cfg" "$spec"
        done
```

---

## Fortress Rollback TLA+ Specs Reference

| Spec | Purpose | Key Properties |
|------|---------|----------------|
| [Rollback.tla](../../specs/tla/Rollback.tla) | Rollback mechanism | Bounded depth, state availability |
| [InputQueue.tla](../../specs/tla/InputQueue.tla) | Input buffering | FIFO order, bounded length |
| [NetworkProtocol.tla](../../specs/tla/NetworkProtocol.tla) | Protocol state machine | Valid transitions, no deadlock |
| [Concurrency.tla](../../specs/tla/Concurrency.tla) | Thread safety | Mutual exclusion, no races |
| [ChecksumExchange.tla](../../specs/tla/ChecksumExchange.tla) | Desync detection | Eventual detection |

---

## Quick Reference: TLA+ Syntax

### Operators

| Operator | Meaning | Example |
|----------|---------|---------|
| `==` | Definition | `Max(a,b) == IF a > b THEN a ELSE b` |
| `=` | Equality test | `x = 5` |
| `/\` | And (conjunction) | `x > 0 /\ y > 0` |
| `\/` | Or (disjunction) | `x = 0 \/ y = 0` |
| `~` | Not | `~inRollback` |
| `=>` | Implies | `inRollback => target < frame` |
| `<=>` | Equivalent | `empty <=> (head = tail)` |
| `'` | Next state (prime) | `x' = x + 1` |
| `[]` | Always (box) | `[]Invariant` |
| `<>` | Eventually (diamond) | `<>Done` |
| `~>` | Leads to | `Request ~> Response` |
| `\in` | Element of | `x \in 1..10` |
| `\notin` | Not element of | `x \notin Banned` |
| `\subseteq` | Subset | `A \subseteq B` |
| `\union` | Set union | `A \union B` |
| `\intersect` | Set intersection | `A \intersect B` |
| `\` | Set difference | `A \ B` |
| `\A` | For all | `\A x \in S: P(x)` |
| `\E` | Exists | `\E x \in S: P(x)` |
| `CHOOSE` | Select element | `CHOOSE x \in S: P(x)` |
| `EXCEPT` | Function update | `[f EXCEPT ![k] = v]` |
| `DOMAIN` | Function domain | `DOMAIN f` |
| `UNCHANGED` | Variables unchanged | `UNCHANGED <<x, y>>` |
| `ENABLED` | Action enabled | `ENABLED Next` |
| `WF_vars(A)` | Weak fairness | Must eventually do A if always enabled |
| `SF_vars(A)` | Strong fairness | Must eventually do A if repeatedly enabled |

### Set Constructions

```tla
{1, 2, 3}                    \* Enumeration
1..10                        \* Range
{x \in S: P(x)}             \* Filter
{f(x): x \in S}             \* Map
SUBSET S                     \* Power set
S \X T                       \* Cartesian product
[S -> T]                     \* Function set (all functions from S to T)
```

### Function/Record Syntax

```tla
[x \in S |-> expr]          \* Function definition
[a |-> 1, b |-> 2]          \* Record literal
f[x]                         \* Function application
r.field                      \* Record field access
[f EXCEPT ![x] = v]         \* Function update
[r EXCEPT !.field = v]      \* Record field update
```

---

## Summary: TLA+ Development Workflow

1. **Identify critical properties** — What must always/eventually be true?
2. **Choose abstraction level** — Model essential behavior, skip implementation details
3. **Write type invariant first** — Documents variable types, catches many errors
4. **Define Init and Next** — Initial state and all possible transitions
5. **Add safety invariants** — Properties that must hold in every state
6. **Run model checker** — Start with tiny constants, increase gradually
7. **Add liveness (if needed)** — Properties that must eventually hold
8. **Iterate on counterexamples** — Each failure reveals spec or design bugs
9. **Maintain alongside code** — Update spec when design changes

---

## References

- [TLA+ Home Page](https://lamport.azurewebsites.net/tla/tla.html) — Leslie Lamport's official site
- [Learn TLA+](https://learntla.com/) — Hillel Wayne's practical tutorial
- [TLA+ Examples](https://github.com/tlaplus/examples) — Official example repository
- [Amazon's Use of TLA+](https://lamport.azurewebsites.net/tla/formal-methods-amazon.pdf) — Industry case study
- [Specifying Systems](https://lamport.azurewebsites.net/tla/book.html) — Lamport's TLA+ book (free online)

---

*This skill document is part of the Fortress Rollback LLM context. See also: defensive-programming.md, type-driven-design.md*
