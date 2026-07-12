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

- Protocol-v1 framing rejects legacy packets, unsupported versions, unknown
  flags, bad sentinels, unknown tags, and malformed fixed-width bodies.
- Decoding checks lengths against remaining bytes before allocation and applies
  a 64 MiB decoded-byte ceiling, configured frame/depth limits, and fallible
  allocation.
- Built-in sockets cap each poll at 256 raw receive attempts and 256 decoded
  messages. Protocol output and recovery queues are bounded.
- A validated 32-bit connection ID filters stale and cross-session traffic
  after synchronization; sync replies must echo an outstanding random token.
- The v1 handshake compares the protocol floor, player count, fixed input
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
of every v1 build. V1 accepts exactly version 1; `min_compat_version` reserves a
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

Packet authentication is deferred from protocol v1. Its reserved flag bit
remains available, while requiring crypto in the core would expand the unsafe,
SIMD, dependency-vetting, and portability surface. Dominant browser
deployments already carry authenticated DTLS, and applications can wrap the
socket boundary today.

## Dishonest-Peer Capabilities

| Capability | Current posture | Residual risk |
| --- | --- | --- |
| Conflicting inputs for one frame | Later checksum divergence can detect, but not attribute, it | Honest peers may diverge; prevention needs stronger agreement or signed evidence |
| False connection-status gossip | Range checks and converge-down merging limit some values | A peer can delay progress or falsely describe a third peer |
| False checksum report | Advisory per-peer trust downgrade; never automatic ejection | A peer can accuse an honest participant; a mismatch does not prove local fault |
| False floor-round report | Range and consumer-state checks reject invalid domains | A valid-looking lie can wedge or release lifecycle holds incorrectly |
| Packet or handshake flood | Bounded decode, allocation, queues, and per-poll work | Kernel/raw-UDP pressure and distributed DDoS remain outside the model |
| Poisoned hot-join snapshot | Desync detection can expose later disagreement | The joiner initially trusts its coordinator's snapshot |
| Cross-session replay | Connection ID plus sync-token echo after binding | Pre-binding sync requests are not replay-protected and can terminally poison a new raw-UDP handshake; use an authenticated transport |
| Source-address spoofing | No pre-binding library authentication | Raw UDP sessions can be interfered with by a capable spoofer |

Checksum reports are evidence of disagreement, not proof of which endpoint is
faulty. Preserve replays and input logs for investigation. A future signed-input
scheme could provide transferable equivocation evidence, but is not part of v1.

## Structural Non-Goals

Deterministic rollback gives every participant the synchronized inputs and game
state needed to simulate the match. Never place secrets in synchronized state:
fog-of-war secrecy, memory-reading resistance, input-reading bots, fixed-delay
or lookahead fairness, collusion, and general anti-cheat are application design
problems. Full Byzantine fault tolerance and membership consensus are also out
of scope; under a partition, applications should pause or void a match when
their own survivor/quorum policy is no longer satisfied.
