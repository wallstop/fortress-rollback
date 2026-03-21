# Ask User Question Template

Use this template when clarification is needed before continuing code review, debugging, planning, or implementation.

## Four-Step Format

1. Acknowledge what you understood.
2. State the exact ambiguity or missing input.
3. Ask one or two focused questions.
4. Offer a recommended default path if no clarification is available.

## Example: Code Review

I understand you want a review of rollback synchronization changes.
I still need to confirm the expected behavior for out-of-order packets.

- Should delayed packets be dropped once prediction has advanced beyond that frame?
- Is the expected behavior the same for P2P and Spectator sessions?

If you prefer, I can review with a fail-closed assumption: reject stale packets and flag any behavioral divergence.

## Example: Debugging

I understand there is intermittent flakiness in network tests.
The missing detail is where the timeout occurs in the flow.

- Does the timeout occur before handshake completion or during rollback replay?
- Is it reproducible only under chaos settings (latency/loss), or also on clean UDP?

If you do not have those details yet, I can start with a timeout-stage trace and narrow from first failing checkpoint.