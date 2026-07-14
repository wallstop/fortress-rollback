<!-- SYNC: This source doc syncs to wiki/Threat-Model.md. -->

# Threat Model

Fortress Rollback is a deterministic rollback protocol for semi-trusted peers.
It validates hostile network bytes and fails closed on incompatible sessions,
but it is not an authentication, confidentiality, anti-cheat, or Byzantine
consensus system. Applications must choose a transport and match policy that
fit their deployment.

## Trust Boundary

The library assumes that configured peers are intended match participants and
normally run the same game logic and session configuration. Network input is
still treated as untrusted: malformed lengths, tags, versions, flags, frame
ranges, and configuration values are rejected before they can drive unbounded
allocation or protocol state.

The peer address supplied by `NonBlockingSocket` is the protocol identity. Raw
UDP does not prove that identity. An attacker able to spoof a configured source
address can inject traffic, including a configuration-bearing sync request
before a connection ID is bound. Use an authenticated transport when source
spoofing or on-path attackers are in scope.

## In-Scope Defenses

- Protocol-v2 framing rejects legacy packets, released v1 packets, unsupported versions, unknown
  flags, bad sentinels, unknown tags, and malformed fixed-width bodies.
- Decoding checks lengths against remaining bytes before allocation and applies
  a 64 MiB decoded-byte ceiling, configured frame/depth limits, and fallible
  allocation.
- Built-in sockets cap each poll at 256 raw receive attempts and 256 decoded
  messages. Persistent pending-output, pending-checksum, input/recovery,
  drop-mailbox, and session-event structures have explicit caps. The transient
  protocol send queue relies on bounded ingress and poll behavior rather than
  an independent queue limit.
- Raw byte-stream adapters can use `codec::FrameDecoder`, which rejects zero or
  over-64-MiB length declarations before payload allocation, buffers at most one
  incomplete frame, and remains poisoned after malformed input until the
  connection is replaced.
- Protocol endpoints count messages at or above a conservative 1,200-byte
  cross-transport budget and warn once per endpoint era. At or above the
  1,472-byte IPv4/UDP fragmentation boundary they also increment a distinct
  counter and emit one alarm. These diagnostics are not path-MTU discovery and
  do not reject an otherwise valid message.
- A socket send is best-effort and may be dropped or delayed by the local
  adapter or a congestion-controlled QUIC sender stack. The redundant input
  window tolerates ordinary omissions; authentication wrappers must preserve
  this non-blocking contract.
- A validated 32-bit connection ID filters stale and cross-session traffic
  after synchronization; sync replies must echo an outstanding random token.
- The v2 handshake compares the protocol floor, player count, fixed input
width, FPS, prediction window, checksum interval, feature bits, and canonical
  configuration digest before an endpoint can run.
- A configuration mismatch is sticky and terminal for that endpoint. It emits
  one `IncompatibleSession` event and stops retry/timeout activity, while still
  replying with its own configuration so the other peer can diagnose it.

The connection ID and random echo reduce accidental replay; they are not
cryptographic authenticators. Their 32-bit filter gives a random off-path
injection probability of 2^-32 per guessed connection ID, before considering
transport protections.

## Configuration Digest

The handshake digest is raw FNV-1a over the exact canonical bytes:

```text
"FRv1-cfg" || num_players:u16le || input_width:u16le || fps:u32le
           || max_prediction:u16le || desync_interval:u32le
           || features:u32le
```

It detects accidental drift and codec mistakes. It is neither collision
resistant nor keyed, so a malicious peer can advertise false fields or forge a
matching digest. Concrete fields are compared before the digest to provide a
stable, useful reason. A digest-only corruption may therefore be diagnosed by
only the receiver of that corrupt packet; ordinary field mismatches are
reported with endpoint-local `ours`/`theirs` orientation on both sides.

`DisconnectBehavior` is intentionally excluded because it is local policy
after a disconnect, not deterministic simulation configuration. Feature bit 0
describes compile-time hot-join wire capability. Floor-round messages are part
of every v2 build. V2 accepts exactly version 2; `min_compat_version` reserves a
future speak-down policy but does not make current versions interoperable.

Any change to bytes a message can produce or accept requires a protocol-version
bump. A tail extension without a bump is allowed only when it is optional for
correctness and send-gated by a negotiated feature bit.

## Delegated Controls

Confidentiality, peer authentication, integrity against on-path modification,
replay protection stronger than protocol connection IDs, DDoS resistance, NAT
traversal, and address migration belong to the transport or application.
WebRTC data channels normally provide DTLS; similarly, an application can wrap
`NonBlockingSocket` with an AEAD or HMAC envelope that authenticates a sequence
number, match identifier, source identity, and encoded Fortress message before
returning the decoded message to the session.

Reserve a unique nonce per authenticated packet, reject duplicate or stale
sequence numbers in a bounded replay window, and bind the configured peer
identity as associated data. Do not add address migration to raw UDP without
packet authentication.

Packet authentication remains deferred in protocol v2. Its reserved flag bit
remains available, while requiring crypto in the core would expand the unsafe,
SIMD, dependency-vetting, and portability surface. Dominant browser
deployments already carry authenticated DTLS, and applications can wrap the
socket boundary today.

A stream length prefix provides boundaries, not authenticity. Apply an AEAD or
HMAC outside the framed Fortress payload when hostile peers or networks are in
scope, and close the connection after any framing or authentication failure.
Likewise, reliable ordered delivery does not remove denial-of-service risk:
TCP and ordered QUIC streams can head-of-line block all later rollback inputs
behind one lost segment. Prefer authenticated unreliable datagrams for live
rollback traffic where the platform permits them.

## Dishonest-Peer Capabilities

This matrix assumes at most one dishonest match participant. The separately
described on-path or source-spoofing attacker is outside that premise unless an
authenticated transport reduces it to a configured participant identity.

| Capability | Current posture | Residual risk |
| --- | --- | --- |
| Conflicting inputs for one frame | With checksum detection enabled and continued progress, downstream divergence can be detected but not attributed | Honest peers may diverge; prevention needs stronger agreement or signed evidence |
| False connection-status gossip | Range checks and converge-down merging bound invalid or inflated values | One valid low report can conservatively pin one observer; the measured one-edge B2 row recovered after the lie stopped |
| False checksum report | Advisory per-peer trust downgrade; never automatic ejection | A peer can accuse an honest participant; a mismatch does not prove local fault |
| False floor-round report | Range, solicitation, sequence, freshness, and consumer-state checks reject malformed or stale reports | One valid low floor can wedge an observer; one valid high floor can release a hold early, bounded by truthful status and other terms in the measured B4 row |
| Packet or handshake flood | Bounded decode, allocation, queues, and per-poll work | Kernel/raw-UDP pressure and distributed DDoS remain outside the model |
| Poisoned hot-join snapshot | Desync detection can expose later disagreement | The joiner initially trusts its coordinator's snapshot |
| Cross-session replay | Connection ID plus sync-token echo after binding | Pre-binding sync requests are not replay-protected and can terminally poison a new raw-UDP handshake; use an authenticated transport |
| Source-address spoofing | No pre-binding library authentication | Raw UDP sessions can be interfered with by a capable spoofer |

Checksum reports are evidence of disagreement, not proof of which endpoint is
faulty. Preserve replays and input logs for investigation. A future signed-input
scheme could provide transferable equivocation evidence, but is not part of v2.

### B1: Equivocation is detected, not prevented

A dishonest participant can send different input bytes for the same player and
frame to different recipients. Fortress does not run a commit-reveal round or
cross-sign every 60 Hz input. If that equivocation creates a state difference
that persists to a scheduled comparison frame and the application supplies
checksums, the checksum exchange can expose disagreement. Detection is not
guaranteed when inputs are semantically equivalent, state reconverges before a
comparison, checksums are omitted, or checksums collide. Even when detected,
the exchange cannot prove which sender or recipient lied. A liar can also send
false checksum reports.

Quarantine or void the match at the application's chosen first-disagreement
boundary. Preserve the replay, build and configuration identity, per-peer input
logs, and authenticated transport packet logs when available. Do not present
one peer's accusation as transferable proof. Applications that require
attribution must add authenticated, frame-bound input evidence or a stronger
agreement protocol outside Fortress; neither is implemented by protocol v2.
Commit-reveal remains deliberately unadopted because its extra rounds add
slowest-peer latency and cryptographic work to the live input path.

### B3: Checksum accusations require corroboration

`peer_checksum_mismatch_count(handle)` counts confirmed-frame reports from one
remote that disagree with local retained history. Ten mismatching comparisons
currently produce one advisory trust-downgrade warning. The warning is a
persistence signal, not a verdict, and the library never ejects the peer. One
underlying divergence can span many confirmed comparisons and cross the
threshold; conversely, a dishonest peer can fabricate every checksum it sends.

Compare the same confirmed frame across independently captured replays, input
logs, builds, and other peers before assigning blame. Agreement among multiple
honest peers is useful operational evidence under this single-dishonest-peer
model, but it is not cryptographic proof and does not extend to collusion. The
[desync incident playbook](desync-playbook.md) defines the quarantine and
evidence-preservation steps.

### B5: Flood resistance ends at the socket boundary

The decoder bounds work and memory per accepted message, built-in sockets
inspect at most 256 raw receive attempts and return at most 256 decoded messages
per poll, and the named persistent protocol/session structures have explicit
limits. With the built-in sockets, those limits keep one poll's ingress work
finite; a custom `NonBlockingSocket` must impose an equivalent finite receive
batch. The transient protocol send queue has no independent limit and instead
relies on bounded ingress/poll behavior. None of these controls guarantees
useful traffic wins admission when every poll is saturated, nor do they bound
kernel queues, interrupt load, link bandwidth, upstream amplification, or a
distributed flood.

Keep polling and draining events under load, alert on sustained cap saturation,
unknown-source traffic, queue high-water marks, and packet loss, and rate-limit
before the `NonBlockingSocket` boundary. Prefer an authenticated transport or
front-end that drops unauthenticated traffic cheaply. Apply OS and network-edge
controls for raw UDP deployments. Raising Fortress queue or receive limits is
capacity tuning, not DDoS mitigation, and can increase the attacker's resource
budget.

### B6: Hot-join coordinators are trusted state authorities

With `hot-join` enabled, a joiner loads the state snapshot supplied by the
selected coordinator. Fortress bounds and validates the encoded snapshot, but
it does not authenticate the state semantics or ask a second survivor to attest
to its digest. A malicious coordinator can therefore start the joiner from a
fabricated state. Later checksum disagreement may reveal the inconsistency; it
cannot undo the initial trust decision or prove who fabricated the state.

Authenticate participant identity and permit snapshot service only from a
coordinator the application trusts for that match. Record the coordinator,
snapshot frame, and build/configuration identity. If the application computes a
fingerprint of the loaded state, retain it as incident evidence. The optional
snapshot checksum is supplied by that same coordinator, and neither it nor an
application-computed fingerprint independently proves provenance. Abort the
join and rebuild from a separately trusted source when provenance is uncertain.
A second-survivor countersignature is a research direction, not a current
defense.

## Structural Non-Goals

Deterministic rollback gives every participant the synchronized inputs and game
state needed to simulate the match. Never place secrets in synchronized state:
fog-of-war secrecy, memory-reading resistance, input-reading bots, fixed-delay
or lookahead fairness, collusion, and general anti-cheat are application design
problems. Full Byzantine fault tolerance and membership consensus are also out
of scope; under a partition, applications should pause or void a match when
their own survivor/quorum policy is no longer satisfied.
