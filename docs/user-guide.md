<p align="center">
  <img src="../assets/logo.svg" alt="Fortress Rollback" width="128">
</p>

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
   - [Network Scenario Configuration Guide](#network-scenario-configuration-guide)
     - [LAN / Local Network](#lan--local-network--20ms-rtt)
     - [Regional Internet](#regional-internet-20-80ms-rtt)
     - [High Latency](#high-latency-80-200ms-rtt)
     - [Lossy Network](#lossy-network-5-15-packet-loss)
     - [Competitive/Tournament](#competitivetournament-strict-requirements)
     - [Casual Multiplayer](#casual-multiplayer-4-players)
     - [Spectator Streaming](#spectator-streaming)
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
use web_time::Duration;

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
use web_time::{Duration, Instant};

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

Checksums enable desync detection. Serialize your state and hash it using the library's codec module:

```rust
use fortress_rollback::network::codec::encode;

fn compute_checksum(state: &GameState) -> u128 {
    let bytes = encode(state).expect("Serialization failed");
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

> **Note:** The `network::codec` module uses a fixed-integer bincode configuration that ensures deterministic serialization across platforms. This is the same configuration used internally for network messages.

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

// Mobile: High tolerance for variable mobile networks
let session = SessionBuilder::<GameConfig>::new()
    .with_sync_config(SyncConfig::mobile())
    // ...

// Competitive: Fast sync with strict timeouts
let session = SessionBuilder::<GameConfig>::new()
    .with_sync_config(SyncConfig::competitive())
    // ...
```

**Preset Comparison:**

| Preset | Sync Packets | Retry Interval | Timeout | Best For |
|--------|--------------|----------------|---------|----------|
| `default()` | 5 | 200ms | None | General internet play |
| `lan()` | 3 | 100ms | 5s | LAN parties, localhost |
| `high_latency()` | 5 | 400ms | 10s | Intercontinental, WiFi |
| `lossy()` | 8 | 200ms | 10s | Unstable connections |
| `mobile()` | 10 | 350ms | 15s | Mobile/cellular networks |
| `competitive()` | 4 | 100ms | 3s | Esports, tournaments |

### Network Scenario Configuration Guide

This section provides complete, production-ready configurations for different network scenarios. Each configuration is designed to optimize the balance between responsiveness and stability for its target environment.

---

#### LAN / Local Network (< 20ms RTT)

Best for: LAN parties, local tournaments, same-building connections.

**Characteristics:**

- Ultra-low latency (~1-20ms RTT)
- No packet loss
- Extremely stable connection

```rust
use fortress_rollback::{
    DesyncDetection, ProtocolConfig, SessionBuilder, SyncConfig,
    TimeSyncConfig,
};
use web_time::Duration;

let session = SessionBuilder::<GameConfig>::new()
    .with_num_players(2)
    // Zero input delay - immediate response
    .with_input_delay(0)
    // Minimal prediction needed
    .with_max_prediction_window(4)
    // LAN presets for fast sync
    .with_sync_config(SyncConfig::lan())
    .with_protocol_config(ProtocolConfig::competitive())
    .with_time_sync_config(TimeSyncConfig::lan())
    // Fast disconnect detection (1 second)
    .with_disconnect_timeout(Duration::from_millis(1000))
    .with_disconnect_notify_delay(Duration::from_millis(200))
    // Frequent desync checks (cheap on LAN)
    .with_desync_detection_mode(DesyncDetection::On { interval: 60 })
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
    .start_p2p_session(socket)?;
```

**Why these settings:**

- `input_delay(0)`: With <20ms RTT, inputs arrive before next frame
- `max_prediction_window(4)`: Small window since predictions rarely wrong
- `SyncConfig::lan()`: 3 sync packets, 100ms retry (fast handshake)
- `TimeSyncConfig::lan()`: 10-frame window (faster adaptation)
- `disconnect_timeout(1000ms)`: Fast detection acceptable on stable network

---

#### Regional Internet (20-80ms RTT)

Best for: Same-country connections, good home internet, regional matchmaking.

**Characteristics:**

- Low-moderate latency (20-80ms RTT)
- Occasional packet loss (<2%)
- Generally stable

```rust
use fortress_rollback::{
    DesyncDetection, ProtocolConfig, SessionBuilder, SyncConfig,
    TimeSyncConfig,
};
use web_time::Duration;

let session = SessionBuilder::<GameConfig>::new()
    .with_num_players(2)
    // Light input delay to reduce rollbacks
    .with_input_delay(2)
    // Standard prediction window
    .with_max_prediction_window(8)
    // Default presets work well for regional
    .with_sync_config(SyncConfig::default())
    .with_protocol_config(ProtocolConfig::default())
    .with_time_sync_config(TimeSyncConfig::default())
    // Standard disconnect handling
    .with_disconnect_timeout(Duration::from_millis(2500))
    .with_disconnect_notify_delay(Duration::from_millis(500))
    // Regular desync checks
    .with_desync_detection_mode(DesyncDetection::On { interval: 100 })
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
    .start_p2p_session(socket)?;
```

**Why these settings:**

- `input_delay(2)`: Masks ~33ms of latency, reduces most rollbacks
- `max_prediction_window(8)`: Handles typical jitter spikes
- `SyncConfig::default()`: Balanced 5-packet handshake

---

#### High Latency (80-200ms RTT)

Best for: Intercontinental connections, WiFi on congested networks, mobile hotspots.

**Characteristics:**

- High latency (80-200ms RTT)
- Moderate packet loss (2-5%)
- Variable jitter

```rust
use fortress_rollback::{
    DesyncDetection, ProtocolConfig, SaveMode, SessionBuilder, SyncConfig,
    TimeSyncConfig,
};
use web_time::Duration;

let session = SessionBuilder::<GameConfig>::new()
    .with_num_players(2)
    // Higher input delay to reduce rollback frequency
    .with_input_delay(4)
    // Large prediction window for latency spikes
    .with_max_prediction_window(12)
    // High-latency presets with longer intervals
    .with_sync_config(SyncConfig::high_latency())
    .with_protocol_config(ProtocolConfig::high_latency())
    .with_time_sync_config(TimeSyncConfig::smooth())
    // Generous disconnect handling
    .with_disconnect_timeout(Duration::from_millis(5000))
    .with_disconnect_notify_delay(Duration::from_millis(2000))
    // Less frequent desync checks (reduce overhead)
    .with_desync_detection_mode(DesyncDetection::On { interval: 150 })
    // Consider sparse saving if rollbacks are long
    .with_save_mode(SaveMode::EveryFrame)
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
    .start_p2p_session(socket)?;
```

**Why these settings:**

- `input_delay(4)`: Masks ~67ms, smooths out most high-latency play
- `max_prediction_window(12)`: Handles 200ms RTT without stalling
- `SyncConfig::high_latency()`: 400ms retry intervals prevent flooding
- `TimeSyncConfig::smooth()`: 60-frame window prevents oscillation
- `disconnect_timeout(5000ms)`: Tolerates temporary connection hiccups

**Tip:** Display the input delay to players so they understand the tradeoff:

```rust
// UI hint: "Input delay: 4 frames (~67ms) for smoother gameplay"
let input_delay_ms = input_delay_frames * (1000 / fps);
```

---

#### Lossy Network (5-15% Packet Loss)

Best for: WiFi with interference, congested networks, some cellular connections.

**Characteristics:**

- Variable latency
- Significant packet loss (5-15%)
- Packet reordering common

```rust
use fortress_rollback::{
    DesyncDetection, InputQueueConfig, ProtocolConfig, SessionBuilder,
    SyncConfig, TimeSyncConfig,
};
use web_time::Duration;

let session = SessionBuilder::<GameConfig>::new()
    .with_num_players(2)
    // Moderate input delay
    .with_input_delay(3)
    // Large prediction window to handle dropped packets
    .with_max_prediction_window(15)
    // Lossy preset with extra sync packets
    .with_sync_config(SyncConfig::lossy())
    .with_protocol_config(ProtocolConfig::default())
    .with_time_sync_config(TimeSyncConfig::smooth())
    // Large input queue for buffering
    .with_input_queue_config(InputQueueConfig::high_latency())
    // Very generous disconnect handling
    .with_disconnect_timeout(Duration::from_millis(6000))
    .with_disconnect_notify_delay(Duration::from_millis(2500))
    // Frequent desync checks (packet loss can cause desyncs)
    .with_desync_detection_mode(DesyncDetection::On { interval: 60 })
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
    .start_p2p_session(socket)?;
```

**Why these settings:**

- `SyncConfig::lossy()`: 8 sync packets ensures reliable handshake
- `max_prediction_window(15)`: Tolerates multiple consecutive dropped packets
- `InputQueueConfig::high_latency()`: 256-frame buffer handles bursts
- `DesyncDetection::On { interval: 60 }`: Catches drift from lost packets early

**Warning:** If packet loss exceeds 15%, rollback networking becomes impractical. Consider showing a network quality warning to users.

---

#### Competitive/Tournament (Strict Requirements)

Best for: Tournament play, ranked matches, esports.

**Characteristics:**

- Requires <100ms RTT for fair play
- Zero tolerance for cheating
- Fastest possible response time

```rust
use fortress_rollback::{
    DesyncDetection, ProtocolConfig, SessionBuilder, SyncConfig,
    TimeSyncConfig,
};
use web_time::Duration;

let session = SessionBuilder::<GameConfig>::new()
    .with_num_players(2)
    // Minimal input delay for competitive edge
    .with_input_delay(1)
    // Moderate prediction window
    .with_max_prediction_window(6)
    // Competitive presets
    .with_sync_config(SyncConfig::lan())  // Fast sync even online
    .with_protocol_config(ProtocolConfig::competitive())
    .with_time_sync_config(TimeSyncConfig::responsive())
    // Fast disconnect detection
    .with_disconnect_timeout(Duration::from_millis(1500))
    .with_disconnect_notify_delay(Duration::from_millis(300))
    // Frequent desync detection to catch cheating
    .with_desync_detection_mode(DesyncDetection::On { interval: 30 })
    // Higher FPS for competitive games
    .with_fps(120)?
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Remote(remote_addr), PlayerHandle::new(1))?
    .start_p2p_session(socket)?;
```

**Why these settings:**

- `input_delay(1)`: Minimal delay, accepts more rollbacks for responsiveness
- `DesyncDetection::On { interval: 30 }`: Catches cheating attempts quickly
- `ProtocolConfig::competitive()`: 100ms quality reports for accurate RTT
- `disconnect_timeout(1500ms)`: Quick forfeit on disconnection

**Recommendation:** Enforce RTT limits in matchmaking:

```rust
// Reject matches with >100ms RTT for competitive play
if estimated_rtt_ms > 100 {
    return Err("Connection too slow for ranked play");
}
```

---

#### Casual Multiplayer (4+ Players)

Best for: Party games, casual online, mixed skill levels.

**Characteristics:**

- Variable player count (2-8 players)
- Mixed network conditions across players
- Prioritizes stability over responsiveness

```rust
use fortress_rollback::{
    DesyncDetection, ProtocolConfig, SaveMode, SessionBuilder, SyncConfig,
    TimeSyncConfig,
};
use web_time::Duration;

let session = SessionBuilder::<GameConfig>::new()
    .with_num_players(4)  // Or up to 8
    // Higher input delay for stability with many players
    .with_input_delay(3)
    // Large prediction window for worst-case latency
    .with_max_prediction_window(12)
    // Default presets work for mixed conditions
    .with_sync_config(SyncConfig::default())
    .with_protocol_config(ProtocolConfig::default())
    .with_time_sync_config(TimeSyncConfig::smooth())
    // Very lenient disconnect handling
    .with_disconnect_timeout(Duration::from_millis(7000))
    .with_disconnect_notify_delay(Duration::from_millis(3000))
    // Less frequent desync checks (performance with many players)
    .with_desync_detection_mode(DesyncDetection::On { interval: 200 })
    // Sparse saving helps with performance
    .with_save_mode(SaveMode::Sparse)
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Remote(remote_addr1), PlayerHandle::new(1))?
    .add_player(PlayerType::Remote(remote_addr2), PlayerHandle::new(2))?
    .add_player(PlayerType::Remote(remote_addr3), PlayerHandle::new(3))?
    .start_p2p_session(socket)?;
```

**Why these settings:**

- `input_delay(3)`: Balances stability across varied connections
- `TimeSyncConfig::smooth()`: Prevents oscillation with many peers
- `SaveMode::Sparse`: Reduces save overhead with more players
- `disconnect_timeout(7000ms)`: Gives players time to reconnect

**Note:** With more players, the slowest connection affects everyone. Consider implementing connection quality indicators per player.

---

#### Spectator Streaming

Best for: Live event streaming, replay viewers, tournament broadcasts.

**Characteristics:**

- One-way data flow (host → spectator)
- Spectators may have varied connections
- Acceptable to be slightly behind live play

```rust
use fortress_rollback::{
    ProtocolConfig, SessionBuilder, SpectatorConfig, SyncConfig,
};
use web_time::Duration;

// Host side: P2P session with spectator support
let host_session = SessionBuilder::<GameConfig>::new()
    .with_num_players(2)
    .with_input_delay(2)
    // Spectator-friendly config: larger buffer for varied viewers
    .with_spectator_config(SpectatorConfig {
        buffer_size: 180,      // 3 seconds at 60 FPS
        catchup_speed: 2,      // 2x speed when behind
        max_frames_behind: 30, // Start catchup at 0.5s behind
        ..Default::default()
    })
    .with_sync_config(SyncConfig::default())
    .add_player(PlayerType::Local, PlayerHandle::new(0))?
    .add_player(PlayerType::Remote(player2_addr), PlayerHandle::new(1))?
    .add_player(PlayerType::Spectator(spectator_addr), PlayerHandle::new(2))?
    .start_p2p_session(socket)?;

// Spectator side: uses high-latency tolerant config
let spectator_session = SessionBuilder::<GameConfig>::new()
    .with_num_players(2)
    .with_sync_config(SyncConfig::high_latency())
    .with_protocol_config(ProtocolConfig::high_latency())
    .with_max_frames_behind(30)?
    .with_catchup_speed(2)?
    .start_spectator_session(host_addr, spectator_socket);
```

**SpectatorConfig presets:**

- `SpectatorConfig::fast_paced()`: 90-frame buffer, 2x catchup (action games)
- `SpectatorConfig::slow_connection()`: 120-frame buffer, tolerant (streaming)
- `SpectatorConfig::local()`: 30-frame buffer, minimal delay (local viewing)

---

### Configuration Decision Tree

Use this guide to choose the right configuration:

```
Is RTT < 20ms?
├─ Yes → Use LAN Configuration
└─ No → Is RTT < 80ms?
         ├─ Yes → Use Regional Configuration
         └─ No → Is RTT < 200ms?
                  ├─ Yes → Use High Latency Configuration
                  └─ No → Warn user, connection may be too slow

Is packet loss > 5%?
├─ Yes → Also apply Lossy Network Configuration
└─ No → Standard configuration is fine

Is this competitive play?
├─ Yes → Use Competitive Configuration, enforce RTT limits
└─ No → Standard configuration with more lenient timeouts

Are there 4+ players?
├─ Yes → Use Casual Multiplayer Configuration
└─ No → Standard 2-player configuration
```

### Dynamic Configuration

For games with matchmaking, consider adjusting configuration based on measured network conditions:

```rust
fn configure_for_network(rtt_ms: u32, packet_loss_percent: f32) -> SessionBuilder<GameConfig> {
    let mut builder = SessionBuilder::<GameConfig>::new()
        .with_num_players(2);

    // Adjust input delay based on RTT
    let input_delay = match rtt_ms {
        0..=20 => 0,
        21..=60 => 1,
        61..=100 => 2,
        101..=150 => 3,
        _ => 4,
    };
    builder = builder.with_input_delay(input_delay);

    // Choose sync config based on conditions
    let sync_config = if packet_loss_percent > 5.0 {
        SyncConfig::lossy()
    } else if rtt_ms > 100 {
        SyncConfig::high_latency()
    } else if rtt_ms < 20 {
        SyncConfig::lan()
    } else {
        SyncConfig::default()
    };
    builder = builder.with_sync_config(sync_config);

    // Adjust prediction window
    let prediction_window = match rtt_ms {
        0..=50 => 6,
        51..=100 => 8,
        101..=150 => 10,
        _ => 12,
    };
    builder = builder.with_max_prediction_window(prediction_window);

    builder
}
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

```rust,ignore
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

## Complete Configuration Reference

This section documents all configuration options available when building a session.

### SessionBuilder Methods

| Method | Default | Description |
|--------|---------|-------------|
| `with_num_players(n)` | 2 | Number of active players (not spectators) |
| `with_input_delay(frames)` | 0 | Frames of input delay for local players |
| `with_max_prediction_window(frames)` | 8 | Max frames ahead without confirmed inputs (0 = lockstep) |
| `with_fps(fps)` | 60 | Expected frames per second for timing |
| `with_save_mode(mode)` | `EveryFrame` | How often to save state for rollback |
| `with_desync_detection_mode(mode)` | `Off` | Checksum comparison between peers |
| `with_disconnect_timeout(duration)` | 2000ms | Time before disconnecting unresponsive peer |
| `with_disconnect_notify_delay(duration)` | 500ms | Time before warning about potential disconnect |
| `with_check_distance(frames)` | 2 | Frames to resimulate in SyncTestSession |
| `with_violation_observer(observer)` | None | Custom observer for spec violations |

### SyncConfig (Synchronization Protocol)

Configure the initial connection handshake with `with_sync_config()`:

```rust,ignore
use fortress_rollback::SyncConfig;

let config = SyncConfig {
    num_sync_packets: 5,                              // Roundtrips required (default: 5)
    sync_retry_interval: Duration::from_millis(200), // Retry interval (default: 200ms)
    sync_timeout: None,                              // Optional timeout (default: None)
    running_retry_interval: Duration::from_millis(200), // Input retry interval (default: 200ms)
    keepalive_interval: Duration::from_millis(200),  // Keepalive interval (default: 200ms)
    ..Default::default()  // Forward compatibility
};
```

**Presets:**

- `SyncConfig::default()` - Balanced for typical internet
- `SyncConfig::lan()` - Fast sync for local networks (3 packets, 100ms intervals)
- `SyncConfig::high_latency()` - Tolerant for 100-200ms RTT (400ms intervals)
- `SyncConfig::lossy()` - Reliable for 5-15% packet loss (8 packets)

### ProtocolConfig (Network Protocol)

Configure network behavior with `with_protocol_config()`:

```rust,ignore
use fortress_rollback::ProtocolConfig;

let config = ProtocolConfig {
    quality_report_interval: Duration::from_millis(200), // RTT measurement interval
    shutdown_delay: Duration::from_millis(5000),         // Cleanup delay after disconnect
    max_checksum_history: 32,                            // Checksums retained for desync
    pending_output_limit: 128,                           // Warning threshold for output queue
    sync_retry_warning_threshold: 10,                    // Warn after N sync retries
    sync_duration_warning_ms: 3000,                      // Warn if sync takes longer
    ..Default::default()
};
```

**Presets:**

- `ProtocolConfig::default()` - General purpose
- `ProtocolConfig::competitive()` - Fast quality reports (100ms), short shutdown
- `ProtocolConfig::high_latency()` - Tolerant thresholds, longer timeouts
- `ProtocolConfig::debug()` - Low thresholds to observe telemetry easily

### TimeSyncConfig (Time Synchronization)

Configure frame advantage averaging with `with_time_sync_config()`:

```rust
use fortress_rollback::TimeSyncConfig;

let config = TimeSyncConfig {
    window_size: 30,  // Frames to average (default: 30)
};
```

**Presets:**

- `TimeSyncConfig::default()` - 30-frame window (0.5s at 60 FPS)
- `TimeSyncConfig::responsive()` - 15-frame window (faster adaptation)
- `TimeSyncConfig::smooth()` - 60-frame window (more stable)
- `TimeSyncConfig::lan()` - 10-frame window (for stable LAN)

### SpectatorConfig (Spectator Sessions)

Configure spectator behavior with `with_spectator_config()`:

```rust
use fortress_rollback::SpectatorConfig;

let config = SpectatorConfig {
    buffer_size: 60,       // Input buffer size in frames (default: 60)
    catchup_speed: 1,      // Frames per step when catching up (default: 1)
    max_frames_behind: 10, // When to start catching up (default: 10)
    ..Default::default()
};
```

**Presets:**

- `SpectatorConfig::default()` - 60-frame buffer, no aggressive catchup
- `SpectatorConfig::fast_paced()` - 90-frame buffer, 2x catchup speed
- `SpectatorConfig::slow_connection()` - 120-frame buffer, tolerant
- `SpectatorConfig::local()` - 30-frame buffer, 2x catchup (minimal latency)

### InputQueueConfig (Input Buffer)

Configure input queue size with `with_input_queue_config()`:

```rust
use fortress_rollback::InputQueueConfig;

let config = InputQueueConfig {
    queue_length: 128,  // Circular buffer size (default: 128)
};
```

**Presets:**

- `InputQueueConfig::default()` - 128 frames (~2.1s at 60 FPS)
- `InputQueueConfig::high_latency()` - 256 frames (~4.3s at 60 FPS)
- `InputQueueConfig::minimal()` - 32 frames (~0.5s at 60 FPS)

**Note:** Maximum input delay is `queue_length - 1`. Call `with_input_queue_config()` before `with_input_delay()` to ensure validation uses the correct limit.

### SaveMode (State Saving)

Configure how states are saved with `with_save_mode()`:

```rust
use fortress_rollback::SaveMode;

// Default: save every frame for minimal rollback distance
builder.with_save_mode(SaveMode::EveryFrame);

// Sparse: only save confirmed frames (fewer saves, longer rollbacks)
builder.with_save_mode(SaveMode::Sparse);
```

### DesyncDetection

Configure checksum comparison with `with_desync_detection_mode()`:

```rust
use fortress_rollback::DesyncDetection;

// Enable with interval (compare checksums every N frames)
builder.with_desync_detection_mode(DesyncDetection::On { interval: 100 });

// Disable (default)
builder.with_desync_detection_mode(DesyncDetection::Off);
```

---

## Error Handling

Fortress Rollback uses `FortressError` for all error conditions. The enum is `#[non_exhaustive]`, so always include a wildcard arm when matching.

### Error Types

| Error | Cause | Recovery |
|-------|-------|----------|
| `PredictionThreshold` | Too far ahead without confirmed inputs | Wait for network to catch up; skip this frame's input |
| `NotSynchronized` | Session not yet synchronized | Keep polling; check `SessionState::Running` before operations |
| `InvalidRequest { info }` | Invalid API usage | Fix code; this is a programming error |
| `InvalidPlayerHandle { handle, max_handle }` | Handle out of range | Use handles 0 to num_players-1 |
| `InvalidFrame { frame, reason }` | Frame number invalid | Check frame is in valid range |
| `MissingInput { player_handle, frame }` | Required input not available | Ensure inputs are added before advancing |
| `MismatchedChecksum { current_frame, mismatched_frames }` | Desync in SyncTestSession | Debug non-determinism |
| `SpectatorTooFarBehind` | Spectator can't catch up | Reconnect spectator |
| `SerializationError { context }` | Serialization failed | Check input/state serialization |
| `SocketError { context }` | Network socket error | Check network, retry connection |
| `InternalError { context }` | Library bug | Please report! |

### Error Handling Patterns

```rust
use fortress_rollback::FortressError;

fn handle_error(error: FortressError) -> Action {
    match error {
        // Recoverable: wait and retry
        FortressError::PredictionThreshold => Action::WaitAndRetry,
        FortressError::NotSynchronized => Action::KeepPolling,

        // Recoverable: reconnect
        FortressError::SpectatorTooFarBehind => Action::Reconnect,
        FortressError::SocketError { .. } => Action::Reconnect,

        // Desync: log and investigate
        FortressError::MismatchedChecksum { current_frame, mismatched_frames } => {
            eprintln!("Desync at frame {}: {:?}", current_frame, mismatched_frames);
            Action::DesyncDetected
        }

        // Programming errors: fix the code
        FortressError::InvalidRequest { info } => panic!("Invalid request: {}", info),
        FortressError::InvalidPlayerHandle { handle, max_handle } => {
            panic!("Invalid player handle {} (max: {})", handle, max_handle)
        }
        FortressError::InvalidFrame { frame, reason } => {
            eprintln!("Invalid frame {}: {}", frame, reason);
            Action::Continue
        }
        FortressError::MissingInput { player_handle, frame } => {
            eprintln!("Missing input for player {} at frame {}", player_handle, frame);
            Action::Continue
        }

        // Fatal errors
        FortressError::SerializationError { context } => {
            eprintln!("Serialization error: {}", context);
            Action::Fatal
        }
        FortressError::InternalError { context } => {
            eprintln!("Internal error (please report): {}", context);
            Action::Fatal
        }

        // Forward compatibility: handle unknown errors
        _ => {
            eprintln!("Unknown error: {}", error);
            Action::Fatal
        }
    }
}
```

### Waiting for Synchronization

```rust
use fortress_rollback::SessionState;
use web_time::{Duration, Instant};

fn wait_for_sync<C: Config>(
    session: &mut P2PSession<C>,
    timeout: Duration,
) -> Result<(), FortressError> {
    let start = Instant::now();

    while session.current_state() != SessionState::Running {
        if start.elapsed() > timeout {
            return Err(FortressError::NotSynchronized);
        }
        session.poll_remote_clients();
        std::thread::sleep(Duration::from_millis(16));
    }
    Ok(())
}
```

### Handling Prediction Threshold

```rust
fn add_input_safe<C: Config>(
    session: &mut P2PSession<C>,
    handle: PlayerHandle,
    input: C::Input,
) -> bool {
    match session.add_local_input(handle, input) {
        Ok(()) => true,
        Err(FortressError::PredictionThreshold) => {
            // Too far ahead - skip this frame's input
            // The game will catch up via rollback
            false
        }
        Err(e) => {
            eprintln!("Input error: {}", e);
            false
        }
    }
}
```

---

## Specification Violations (Telemetry)

Fortress Rollback includes a telemetry system for monitoring internal specification violations. These are issues that don't necessarily cause errors but indicate unexpected behavior.

### Violation Severity Levels

| Severity | Description | Action |
|----------|-------------|--------|
| `Warning` | Unexpected but recoverable | Monitor; may indicate network issues |
| `Error` | Serious issue, degraded behavior | Investigate; may affect gameplay |
| `Critical` | Invariant broken, state may be corrupt | Debug immediately |

### Violation Categories (ViolationKind)

| Kind | Description |
|------|-------------|
| `FrameSync` | Frame counter mismatch or unexpected frame values |
| `InputQueue` | Gap in input sequence, double-confirmation |
| `StateManagement` | Loading non-existent state, checksum issues |
| `NetworkProtocol` | Unexpected message, protocol state errors |
| `ChecksumMismatch` | Local/remote checksum difference |
| `Configuration` | Invalid parameter combinations |
| `InternalError` | Library bugs (please report) |
| `Invariant` | Runtime invariant check failed |
| `Synchronization` | Excessive sync retries, slow sync |

### Setting Up a Violation Observer

```rust
use fortress_rollback::{
    SessionBuilder, Config,
    telemetry::{ViolationObserver, CollectingObserver, SpecViolation},
};
use std::sync::Arc;

// For testing: collect violations for assertions
let observer = Arc::new(CollectingObserver::new());
let session = SessionBuilder::<MyConfig>::new()
    .with_num_players(2)
    .with_violation_observer(observer.clone())
    // ... other config
    .start_p2p_session(socket)?;

// After operations, check for violations
if !observer.is_empty() {
    for violation in observer.violations() {
        eprintln!("Violation: {}", violation);
    }
}
```

**Important:** `CollectingObserver` accumulates violations indefinitely. Call `observer.clear()` periodically to prevent unbounded memory growth in long-running sessions:

```rust
// Check and clear violations periodically (e.g., every N frames)
if frame % 1000 == 0 {
    let violations = observer.violations();
    if !violations.is_empty() {
        log::warn!("Found {} violations in last 1000 frames", violations.len());
        observer.clear();
    }
}
```

### Custom Violation Observer

```rust
use fortress_rollback::telemetry::{ViolationObserver, SpecViolation, ViolationSeverity};

struct MetricsObserver {
    // Your metrics client
}

impl ViolationObserver for MetricsObserver {
    fn on_violation(&self, violation: &SpecViolation) {
        // Send to metrics system
        match violation.severity {
            ViolationSeverity::Warning => {
                // Increment warning counter
            }
            ViolationSeverity::Error | ViolationSeverity::Critical => {
                // Alert on-call
            }
        }
    }
}
```

### Interpreting Common Violations

**Synchronization violations (excessive retries):**

```
[warning/synchronization] Sync retry count high: 15 retries
```

This indicates packet loss during connection. Consider using `SyncConfig::lossy()`.

**Frame sync violations:**

```
[warning/frame_sync] TimeSync::advance_frame called with invalid frame
```

This can occur during initialization edge cases. Usually recovers automatically.

**Input queue violations:**

```
[error/input_queue] Input queue gap detected
```

Indicates network issues causing missed inputs. May cause prediction errors.

### Default Behavior

If no observer is set, violations are logged via the `tracing` crate:

- `Warning` → `tracing::warn!`
- `Error` → `tracing::error!`
- `Critical` → `tracing::error!` with additional context

Enable tracing to see violations:

```rust,ignore
tracing_subscriber::init();
```

### Structured Output for Log Aggregation

The `TracingObserver` outputs all fields as structured tracing fields, making it compatible with JSON logging formatters and log aggregation systems:

```rust,ignore
// Example: Using tracing-subscriber with JSON output
use tracing_subscriber::fmt::format::FmtSpan;

tracing_subscriber::fmt()
    .json() // Enable JSON output
    .init();
```

This produces machine-parseable output like:

```json
{"timestamp":"2024-01-15T12:00:00Z","level":"WARN","severity":"warning","kind":"frame_sync","location":"sync.rs:42","frame":"100","context":"{expected=50, actual=100}","message":"frame mismatch"}
```

### JSON Serialization for Programmatic Access

All telemetry types implement `serde::Serialize` for direct JSON serialization:

```rust,ignore
use fortress_rollback::telemetry::{SpecViolation, ViolationSeverity, ViolationKind};
use fortress_rollback::Frame;

let violation = SpecViolation::new(
    ViolationSeverity::Warning,
    ViolationKind::FrameSync,
    "frame mismatch detected",
    "sync.rs:42",
).with_frame(Frame::new(100))
 .with_context("expected", "50")
 .with_context("actual", "100");

// Direct JSON serialization
let json = serde_json::to_string(&violation).unwrap();

// Or use the convenience method
let json = violation.to_json().unwrap();
let json_pretty = violation.to_json_pretty().unwrap();
```

Example JSON output:

```json
{
  "severity": "warning",
  "kind": "frame_sync",
  "message": "frame mismatch detected",
  "location": "sync.rs:42",
  "frame": 100,
  "context": {
    "actual": "100",
    "expected": "50"
  }
}
```

Key serialization behaviors:

- `severity` and `kind` are serialized as snake_case strings (`"warning"`, `"frame_sync"`)
- `frame` is serialized as an integer for valid frames, or `null` for `None`/`Frame::NULL`
- `context` is serialized as a JSON object with string keys and values

---

## Next Steps

- Read the [Architecture Guide](architecture.md) for deeper understanding
- Check the examples in `examples/ex_game/` for working code
- See `examples/configuration.rs` for configuration patterns
- See `examples/error_handling.rs` for error handling patterns
- Join the GGPO Discord for community support
- File issues at the project repository

Happy rollback networking!
