# Fortress Rollback User Guide

This guide walks you through integrating Fortress Rollback into your game. By the end, you'll understand how to set up sessions, handle inputs, manage game state, and respond to network events.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Defining Your Config](#defining-your-config)
3. [Setting Up a P2P Session](#setting-up-a-p2p-session)
4. [The Game Loop](#the-game-loop)
5. [Handling Requests](#handling-requests)
6. [Handling Events](#handling-events)
7. [Determinism Requirements](#determinism-requirements)
8. [Network Requirements](#network-requirements)
9. [Advanced Configuration](#advanced-configuration)
10. [Spectator Sessions](#spectator-sessions)
11. [Testing with SyncTest](#testing-with-synctest)
12. [Common Patterns](#common-patterns)
13. [Troubleshooting](#troubleshooting)

---

## Quick Start

Here's a minimal example to get you started:

```rust
use fortress_rollback::{
    Config, FortressRequest, Frame, InputStatus, NonBlockingSocket,
    PlayerHandle, PlayerType, SessionBuilder, SessionState,
    UdpNonBlockingSocket,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

// 1. Define your input type
#[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
struct MyInput {
    buttons: u8,
}

// 2. Define your game state (must be Clone + Serialize/Deserialize for checksums)
#[derive(Clone, Serialize, Deserialize)]
struct MyGameState {
    frame: i32,
    player_x: f32,
    player_y: f32,
}

// 3. Create your config type
struct MyConfig;
impl Config for MyConfig {
    type Input = MyInput;
    type State = MyGameState;
    type Address = SocketAddr;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 4. Create a session
    let socket = UdpNonBlockingSocket::bind_to_port(7000)?;
    let remote_addr: SocketAddr = "127.0.0.1:7001".parse()?;

    let mut session = SessionBuilder::<MyConfig>::new()
        .with_num_players(2)
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    // 5. Game loop
    let mut game_state = MyGameState {
        frame: 0,
        player_x: 0.0,
        player_y: 0.0,
    };

    loop {
        // Poll for network messages
        session.poll_remote_clients();

        // Only process frames when synchronized
        if session.current_state() == SessionState::Running {
            // Add local input
            let input = MyInput { buttons: 0 }; // Get real input here
            session.add_local_input(PlayerHandle::new(0), input)?;

            // Advance the frame
            for request in session.advance_frame()? {
                match request {
                    FortressRequest::SaveGameState { cell, frame } => {
                        cell.save(frame, Some(game_state.clone()), None);
                    }
                    FortressRequest::LoadGameState { cell, .. } => {
                        game_state = cell.load().expect("State should exist");
                    }
                    FortressRequest::AdvanceFrame { inputs } => {
                        // Apply inputs to your game state
                        game_state.frame += 1;
                        // ... update game_state based on inputs
                    }
                }
            }
        }

        // Render and sleep...
    }
}
```

---

## Defining Your Config

The `Config` trait bundles all type parameters for your session:

```rust
use fortress_rollback::Config;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

// Your input type - sent over the network
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GameInput {
    pub buttons: u8,
    pub stick_x: i8,
    pub stick_y: i8,
}

// Your game state - saved and loaded during rollback
#[derive(Clone, Serialize, Deserialize)]
pub struct GameState {
    pub frame: i32,
    pub players: Vec<PlayerState>,
    // ... all your game data
}

// Your config type
pub struct GameConfig;

impl Config for GameConfig {
    type Input = GameInput;
    type State = GameState;
    type Address = SocketAddr; // Or your custom address type
}
```

### Input Type Requirements

Your input type must:
- Be `Copy + Clone + PartialEq`
- Implement `Default` (used for disconnected players)
- Implement `Serialize + Deserialize` (for network transmission)

**Tips:**
- Keep inputs small; they're sent every frame
- Use bitflags for button states
- Consider `#[repr(C)]` for consistent serialization

### State Type Requirements

Your state type must:
- Be `Clone` (for saving/loading)

**Optional but recommended:**
- Implement `Serialize + Deserialize` for checksums

---

## Setting Up a P2P Session

Use `SessionBuilder` to configure and create sessions:

```rust
use fortress_rollback::{
    DesyncDetection, PlayerHandle, PlayerType, SessionBuilder,
    UdpNonBlockingSocket,
};
use instant::Duration;

let socket = UdpNonBlockingSocket::bind_to_port(7000)?;
let remote_addr = "192.168.1.100:7000".parse()?;

let mut session = SessionBuilder::<GameConfig>::new()
    // Number of active players (not spectators)
    .with_num_players(2)

    // Frames of input delay (reduces rollbacks, adds latency)
    .with_input_delay(2)

    // How many frames ahead we can predict
    .with_max_prediction_window(8)

    // Expected frames per second
    .with_fps(60)?

    // Enable desync detection (compare checksums every 100 frames)
    .with_desync_detection_mode(DesyncDetection::On { interval: 100 })

    // Network timeouts
    .with_disconnect_timeout(Duration::from_millis(3000))
    .with_disconnect_notify_delay(Duration::from_millis(500))

    // Add players
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?

    // Start the session
    .start_p2p_session(socket)?;
```

### Understanding Input Delay

Input delay trades responsiveness for smoothness:

| Delay | Effect |
|-------|--------|
| 0 | Immediate response, frequent rollbacks |
| 2 | Slight delay, fewer rollbacks |
| 4+ | Noticeable delay, rare rollbacks |

A delay of 2 frames is a good starting point for most games.

### Lockstep Mode

Set `max_prediction_window(0)` for lockstep networking:

```rust
let session = SessionBuilder::<GameConfig>::new()
    .with_max_prediction_window(0) // Lockstep mode
    .with_input_delay(0)           // No delay needed
    // ...
```

In lockstep mode:
- No rollbacks ever occur
- No save/load requests
- Frame rate limited by slowest connection
- Good for turn-based or slower-paced games

---

## The Game Loop

A typical game loop with Fortress Rollback:

```rust
use instant::{Duration, Instant};

const FPS: f64 = 60.0;
let frame_duration = Duration::from_secs_f64(1.0 / FPS);

let mut last_update = Instant::now();
let mut accumulator = Duration::ZERO;

loop {
    // 1. Network polling (do this frequently)
    session.poll_remote_clients();

    // 2. Handle events
    for event in session.events() {
        handle_event(event);
    }

    // 3. Fixed timestep accumulator
    let now = Instant::now();
    accumulator += now - last_update;
    last_update = now;

    // 4. Adjust for frame advantage (optional, helps sync)
    let mut adjusted_duration = frame_duration;
    if session.frames_ahead() > 0 {
        adjusted_duration = Duration::from_secs_f64(1.0 / FPS * 1.1);
    }

    // 5. Process frames
    while accumulator >= adjusted_duration {
        accumulator -= adjusted_duration;

        if session.current_state() == SessionState::Running {
            // Add input for all local players
            for handle in session.local_player_handles() {
                let input = get_local_input(handle);
                session.add_local_input(handle, input)?;
            }

            // Advance and handle requests
            let requests = session.advance_frame()?;
            handle_requests(requests, &mut game_state);
        }
    }

    // 6. Render
    render(&game_state);

    // 7. Sleep/wait
    std::thread::sleep(Duration::from_millis(1));
}
```

### Important: Order Matters

1. Call `poll_remote_clients()` before checking state or adding input
2. Add input for **all** local players before calling `advance_frame()`
3. Process **all** requests in the order received

---

## Handling Requests

Requests are returned by `advance_frame()` and must be processed in order:

```rust
fn handle_requests(
    requests: Vec<FortressRequest<GameConfig>>,
    game_state: &mut GameState,
) {
    for request in requests {
        match request {
            FortressRequest::SaveGameState { cell, frame } => {
                // Verify frame matches
                assert_eq!(game_state.frame, frame.as_i32());

                // Clone your state
                let state_copy = game_state.clone();

                // Optionally compute a checksum
                let checksum = compute_checksum(game_state);

                // Save it
                cell.save(frame, Some(state_copy), Some(checksum));
            }

            FortressRequest::LoadGameState { cell, frame } => {
                // Load the saved state
                *game_state = cell.load().expect("State should exist");

                // Optionally verify frame
                assert_eq!(game_state.frame, frame.as_i32());
            }

            FortressRequest::AdvanceFrame { inputs } => {
                // Process inputs for all players
                for (player_idx, (input, status)) in inputs.iter().enumerate() {
                    match status {
                        InputStatus::Confirmed => {
                            // This input is definitely correct
                        }
                        InputStatus::Predicted => {
                            // This input might be wrong (rollback may follow)
                        }
                        InputStatus::Disconnected => {
                            // Player disconnected; input is default value
                            // You might want to use AI or freeze this player
                        }
                    }
                    apply_input(game_state, player_idx, *input, *status);
                }

                // Advance your frame counter
                game_state.frame += 1;
            }
        }
    }
}
```

### Computing Checksums

Checksums enable desync detection. Serialize your state and hash it:

```rust
fn compute_checksum(state: &GameState) -> u128 {
    let bytes = bincode::serialize(state).expect("Serialization failed");
    fletcher16(&bytes) as u128
}

fn fletcher16(data: &[u8]) -> u16 {
    let mut sum1: u16 = 0;
    let mut sum2: u16 = 0;
    for byte in data {
        sum1 = (sum1 + *byte as u16) % 255;
        sum2 = (sum2 + sum1) % 255;
    }
    (sum2 << 8) | sum1
}
```

---

## Handling Events

Events notify you of network conditions:

```rust
use fortress_rollback::FortressEvent;

fn handle_event(event: FortressEvent<GameConfig>) {
    match event {
        FortressEvent::Synchronizing {
            addr,
            total,
            count,
            total_requests_sent,
            elapsed_ms,
        } => {
            println!("Syncing with {}: {}/{}", addr, count, total);
            // High total_requests_sent indicates packet loss during sync
            if total_requests_sent > count * 2 {
                println!("Warning: sync retries detected, possible packet loss");
            }
            // Monitor sync duration for network quality assessment
            if elapsed_ms > 2000 {
                println!("Warning: sync taking {}ms", elapsed_ms);
            }
        }

        FortressEvent::Synchronized { addr } => {
            println!("Synchronized with {}", addr);
        }

        FortressEvent::Disconnected { addr } => {
            println!("Disconnected from {}", addr);
            // Handle disconnection (show UI, pause game, etc.)
        }

        FortressEvent::NetworkInterrupted { addr, disconnect_timeout } => {
            println!(
                "Connection to {} interrupted, disconnecting in {}ms",
                addr, disconnect_timeout
            );
        }

        FortressEvent::NetworkResumed { addr } => {
            println!("Connection to {} resumed", addr);
        }

        FortressEvent::WaitRecommendation { skip_frames } => {
            println!("Recommendation: wait {} frames", skip_frames);
            // Optionally slow down to let others catch up
        }

        FortressEvent::DesyncDetected {
            frame,
            local_checksum,
            remote_checksum,
            addr,
        } => {
            eprintln!(
                "DESYNC at frame {} with {}! Local: {}, Remote: {}",
                frame, addr, local_checksum, remote_checksum
            );
            // This is bad! Debug your determinism.
        }
    }
}
```

---

## Determinism Requirements

**Rollback networking requires deterministic simulation.** The same inputs must always produce the same outputs.

### Common Determinism Issues

| Issue | Solution |
|-------|----------|
| Floating-point differences | Use fixed-point math, or be very careful |
| Random numbers | Use seeded RNG, sync seed across clients |
| HashMap iteration order | Use `BTreeMap` instead |
| System time | Only use frame number, not wall clock |
| Uninitialized memory | Initialize all state |
| Different library versions | Ensure all clients use same code |

### Testing Determinism

Use `SyncTestSession` to verify your game is deterministic:

```rust
let mut session = SessionBuilder::<GameConfig>::new()
    .with_num_players(1)
    .with_check_distance(2)  // How many frames to resimulate
    .start_synctest_session()?;

// Run your game loop
// Session will rollback every frame and compare checksums
// Mismatches indicate non-determinism!
```

---

## Network Requirements

Rollback networking works best under certain network conditions. Understanding these requirements helps you configure Fortress Rollback appropriately and set player expectations.

### Supported Network Conditions

| Condition | Supported Range | Optimal | Notes |
|-----------|-----------------|---------|-------|
| **Round-Trip Time (RTT)** | <200ms | <100ms | Higher RTT = more rollbacks |
| **Packet Loss** | <15% | <5% | Above 15% causes frequent desyncs |
| **Jitter** | <50ms | <20ms | High jitter causes prediction failures |
| **Bandwidth** | >56 kbps | >256 kbps | Per-connection requirement |

### Condition Effects

**Low Latency (LAN, <20ms RTT)**
- Minimal rollbacks
- Very responsive gameplay
- Use `SyncConfig::lan()` preset for faster connection

**Medium Latency (Regional, 20-80ms RTT)**
- Occasional rollbacks
- Generally smooth gameplay
- Default configuration works well

**High Latency (Intercontinental, 80-200ms RTT)**
- Frequent rollbacks
- Noticeable input delay recommended (2-3 frames)
- Use `SyncConfig::high_latency()` preset
- Consider increasing `max_prediction_frames`

**Very High Latency (>200ms RTT)**
- May experience frequent sync failures
- Gameplay quality significantly degraded
- Not recommended for competitive play

### Conditions to Avoid

| Condition | Problem | Mitigation |
|-----------|---------|------------|
| Packet loss >15% | Frequent sync failures, desyncs | Use wired connection, improve network |
| Jitter >50ms | Prediction failures, stuttering | QoS settings, reduce network congestion |
| Asymmetric routes | One player experiences more rollbacks | Cannot mitigate at application level |
| NAT traversal issues | Connection failures | Use STUN/TURN, port forwarding |
| Mobile networks | High variability | WiFi recommended over cellular |

### SyncConfig Presets

Fortress Rollback provides configuration presets for different network scenarios:

```rust
use fortress_rollback::{SessionBuilder, SyncConfig};

// Default: Balanced for typical internet connections
let session = SessionBuilder::<GameConfig>::new()
    .with_sync_config(SyncConfig::default())
    // ...

// LAN: Fast connection for local networks
let session = SessionBuilder::<GameConfig>::new()
    .with_sync_config(SyncConfig::lan())
    // ...

// High Latency: More tolerant for 100-200ms RTT connections
let session = SessionBuilder::<GameConfig>::new()
    .with_sync_config(SyncConfig::high_latency())
    // ...

// Lossy: More retries for 5-15% packet loss environments
let session = SessionBuilder::<GameConfig>::new()
    .with_sync_config(SyncConfig::lossy())
    // ...
```

**Preset Comparison:**

| Preset | Sync Packets | Retry Interval | Timeout | Best For |
|--------|--------------|----------------|---------|----------|
| `default()` | 5 | 200ms | None | General internet play |
| `lan()` | 3 | 100ms | 5s | LAN parties, localhost |
| `high_latency()` | 5 | 400ms | 10s | Intercontinental, WiFi |
| `lossy()` | 8 | 200ms | 10s | Unstable connections |

### Tuning Recommendations

**For High Latency:**
```rust
let session = SessionBuilder::<GameConfig>::new()
    .with_sync_config(SyncConfig::high_latency())
    .with_input_delay(3)              // Hide latency with input delay
    .with_max_prediction_frames(12)   // Allow more prediction
    .with_disconnect_timeout(Duration::from_secs(5))
    // ...
```

**For Lossy Networks:**
```rust
let session = SessionBuilder::<GameConfig>::new()
    .with_sync_config(SyncConfig::lossy())
    .with_desync_detection(DesyncDetection::On { interval: 30 })
    // ...
```

**For LAN/Local Play:**
```rust
let session = SessionBuilder::<GameConfig>::new()
    .with_sync_config(SyncConfig::lan())
    .with_input_delay(0)              // No delay needed
    .with_max_prediction_frames(4)    // Less prediction needed
    // ...
```

### Network Quality Monitoring

Monitor network quality using session statistics:

```rust
// Check network stats for each remote player
for handle in session.remote_player_handles() {
    if let Ok(stats) = session.network_stats(handle) {
        let rtt = stats.ping;           // Round-trip time in ms
        let pending = stats.send_queue_len;  // Pending packets

        // Warn if conditions are degrading
        if rtt > 150 {
            println!("Warning: High latency ({}ms) with player {:?}", rtt, handle);
        }

        if pending > 10 {
            println!("Warning: Network congestion with player {:?}", handle);
        }
    }
}
```

### Sync Failure Troubleshooting

If synchronization repeatedly fails:

1. **Check RTT**: If >200ms, use `SyncConfig::high_latency()`
2. **Check packet loss**: If high, use `SyncConfig::lossy()`
3. **Check firewall/NAT**: Ensure UDP traffic is allowed
4. **Monitor sync events**: Watch `total_requests_sent` and `elapsed_ms`

```rust
FortressEvent::Synchronizing { total_requests_sent, elapsed_ms, .. } => {
    // High retry count indicates packet loss
    if total_requests_sent > 15 {
        println!("Warning: excessive sync retries, check network");
    }
    // Long sync time indicates high latency
    if elapsed_ms > 3000 {
        println!("Warning: sync taking {}ms, check connection", elapsed_ms);
    }
}
```

---

## Advanced Configuration

### Sparse Saving

If saving state is expensive, enable sparse saving:

```rust
let session = SessionBuilder::<GameConfig>::new()
    .with_sparse_saving_mode(true)
    // ...
```

With sparse saving:
- Only saves at confirmed frames
- Fewer save requests
- Potentially longer rollbacks

### Custom Sockets

Implement `NonBlockingSocket` for custom networking:

```rust
use fortress_rollback::{Message, NonBlockingSocket};

struct MyCustomSocket { /* ... */ }

impl NonBlockingSocket<MyAddress> for MyCustomSocket {
    fn send_to(&mut self, msg: &Message, addr: &MyAddress) {
        // Serialize and send the message
    }

    fn receive_all_messages(&mut self) -> Vec<(MyAddress, Message)> {
        // Return all received messages since last call
    }
}
```

### ChaosSocket for Testing

Test network resilience with `ChaosSocket`:

```rust
use fortress_rollback::{ChaosConfigBuilder, ChaosSocket, UdpNonBlockingSocket};

let inner_socket = UdpNonBlockingSocket::bind_to_port(7000)?;

let chaos_config = ChaosConfigBuilder::default()
    .latency(50)           // 50ms base latency
    .jitter(20)            // +/- 20ms jitter
    .packet_loss(0.05)     // 5% packet loss
    .build();

let socket = ChaosSocket::new(inner_socket, chaos_config);
```

---

## Spectator Sessions

Spectators observe gameplay without contributing inputs:

### Host Side (P2P Session)

```rust
let spectator_addr = "192.168.1.200:8000".parse()?;

let session = SessionBuilder::<GameConfig>::new()
    .with_num_players(2)
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Remote(player2_addr), PlayerHandle::new(1))?
    // Add spectator with handle >= num_players
    .add_player(PlayerType::Spectator(spectator_addr), PlayerHandle::new(2))?
    .start_p2p_session(socket)?;
```

### Spectator Side

```rust
let host_addr = "192.168.1.100:7000".parse()?;
let socket = UdpNonBlockingSocket::bind_to_port(8000)?;

let mut session = SessionBuilder::<GameConfig>::new()
    .with_num_players(2)
    .with_max_frames_behind(10)?  // When to start catching up
    .with_catchup_speed(2)?       // How fast to catch up
    .start_spectator_session(host_addr, socket);

// Spectator loop
loop {
    session.poll_remote_clients();

    for event in session.events() {
        // Handle sync events
    }

    if session.current_state() == SessionState::Running {
        for request in session.advance_frame()? {
            // Handle requests (no save/load, only AdvanceFrame)
        }
    }
}
```

---

## Testing with SyncTest

`SyncTestSession` helps verify determinism:

```rust
let mut session = SessionBuilder::<GameConfig>::new()
    .with_num_players(2)
    .with_check_distance(4)  // Compare last 4 frames
    .with_input_delay(2)
    .start_synctest_session()?;

// Add players
// Note: All players are local in sync test
session.add_player(PlayerType::Local, PlayerHandle::new(0))?;
session.add_player(PlayerType::Local, PlayerHandle::new(1))?;

// Run game loop
for frame in 0..1000 {
    // Provide input for all players
    for handle in 0..2 {
        session.add_local_input(
            PlayerHandle::new(handle),
            random_input() // Use deterministic "random" for testing
        )?;
    }

    let requests = session.advance_frame()?;
    handle_requests(requests, &mut game_state);
}
```

If checksums mismatch, you have a determinism bug!

---

## Common Patterns

### Handling Disconnected Players

```rust
FortressRequest::AdvanceFrame { inputs } => {
    for (i, (input, status)) in inputs.iter().enumerate() {
        if *status == InputStatus::Disconnected {
            // Option 1: Freeze the player
            continue;

            // Option 2: Simple AI
            // let ai_input = compute_ai_input(&game_state, i);
            // apply_input(&mut game_state, i, ai_input);

            // Option 3: Last known input (already done by Fortress Rollback)
            // apply_input(&mut game_state, i, *input);
        } else {
            apply_input(&mut game_state, i, *input);
        }
    }
}
```

### Multiple Local Players (Couch Co-op)

```rust
let session = SessionBuilder::<GameConfig>::new()
    .with_num_players(4)
    // Two local players, two remote
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Local, PlayerHandle::new(1))?
    .add_player(PlayerType::Remote(addr1), PlayerHandle::new(2))?
    .add_player(PlayerType::Remote(addr2), PlayerHandle::new(3))?
    .start_p2p_session(socket)?;

// In game loop, add input for BOTH local players
for handle in session.local_player_handles() {
    let input = get_input_for_player(handle);
    session.add_local_input(handle, input)?;
}
```

### Frame Pacing

Slow down when ahead to reduce rollbacks:

```rust
let base_fps = 60.0;
let frame_time = 1.0 / base_fps;

let adjusted_time = if session.frames_ahead() > 2 {
    frame_time * 1.1 // Slow down 10%
} else if session.frames_ahead() < -2 {
    frame_time * 0.9 // Speed up 10% (be careful!)
} else {
    frame_time
};
```

---

## Troubleshooting

### "NotSynchronized" Error

**Cause:** Trying to advance frame before synchronization completes.

**Fix:** Check `session.current_state() == SessionState::Running` before adding input or advancing.

### Desync Detected

**Cause:** Non-deterministic game simulation.

**Debug:**
1. Use `SyncTestSession` to reproduce locally
2. Check for HashMap usage, random numbers, floating-point edge cases
3. Ensure all clients run identical code
4. Verify all state is saved/loaded correctly

### Connection Timeout

**Cause:** Network issues or firewall blocking UDP.

**Fix:**
- Verify both peers can reach each other
- Check firewalls allow UDP on your port
- Increase `disconnect_timeout` for flaky connections
- Ensure `poll_remote_clients()` is called frequently

### Rollbacks Too Frequent

**Cause:** High latency or low input delay.

**Fix:**
- Increase `with_input_delay()`
- Consider using sparse saving if saves are slow
- Check network quality

### Game Stutters

**Cause:** Variable frame times or slow save/load.

**Fix:**
- Use fixed timestep game loop
- Profile save/load operations
- Consider sparse saving mode
- Ensure `advance_frame()` completes quickly

### "Input dropped" / NULL_FRAME returned

**Cause:** Input provided for wrong frame or out of sequence.

**Fix:**
- Always add input for current frame only
- Don't skip frames when adding input
- Check you're not calling `add_local_input` multiple times per frame

---

## Next Steps

- Read the [Architecture Guide](ARCHITECTURE.md) for deeper understanding
- Check the examples in `examples/ex_game/` for working code
- Join the GGPO Discord for community support
- File issues at the project repository

Happy rollback networking!
