<p align="center">
  <img src="assets/logo.svg" alt="Fortress Rollback" width="160">
</p>

# Fortress Rollback

## *Deterministic Rollback Netcode Built on Correctness*

Fortress Rollback is a **correctness-first** Rust library for peer-to-peer rollback networking in deterministic multiplayer games. Built as a hardened fork of GGRS (Good Game Rollback System), it prioritizes safety, formal verification, and production reliability.

---

## Key Features

- **Zero-panic guarantee** — All errors returned as `Result`, never crashes
- **Formally verified** — TLA+, Z3, and Kani proofs for critical components
- **Deterministic by design** — `BTreeMap`/`BTreeSet` ensure reproducible behavior
- **Production tested** — 1100+ tests with ~92% code coverage
- **GGRS compatible** — Drop-in replacement with enhanced safety

---

## Quick Start

Get up and running with Fortress Rollback in minutes.

### Cargo.toml

```toml
[dependencies]
fortress_rollback = "0.11"
serde = { version = "1.0", features = ["derive"] }
```

### Basic Session

```rust
use fortress_rollback::{
    Config, FortressRequest, PlayerHandle, PlayerType,
    SessionBuilder, UdpNonBlockingSocket,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

// Define your input and state types
#[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
struct MyInput { buttons: u8 }

#[derive(Clone, Serialize, Deserialize)]
struct MyGameState { frame: i32, /* your state */ }

// Configure Fortress Rollback
struct MyConfig;
impl Config for MyConfig {
    type Input = MyInput;
    type State = MyGameState;
    type Address = SocketAddr;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a UDP socket and session
    let socket = UdpNonBlockingSocket::bind_to_port(7000)?;
    let remote: SocketAddr = "127.0.0.1:7001".parse()?;

    let mut session = SessionBuilder::<MyConfig>::new()
        .with_num_players(2)?
        .add_player(PlayerType::Local, PlayerHandle::new(0))?
        .add_player(PlayerType::Remote(remote), PlayerHandle::new(1))?
        .start_p2p_session(socket)?;

    // Your game loop handles FortressRequests...
    Ok(())
}
```

---

## Where to Go Next

- **[User Guide](User-Guide)** — Complete walkthrough of sessions, inputs, state management, and network events
- **[Architecture](Architecture)** — Deep dive into the internal design and components
- **[Migration Guide](Migration)** — Migrate from GGRS to Fortress Rollback
- **[API Contracts](API-Contracts)** — Formal specifications and guarantees

---

> **Fork of GGRS**
>
> Fortress Rollback is a hardened fork of [GGRS](https://github.com/gschup/ggrs) (Good Game Rollback System). It maintains API compatibility where possible while adding formal verification, eliminating panics, and fixing determinism bugs.
>
> **Key improvements over GGRS:**
>
> - All `panic!` and `assert!` converted to recoverable errors
> - Deterministic `BTreeMap`/`BTreeSet` instead of `HashMap`/`HashSet`
> - 1100+ tests with ~92% code coverage
> - TLA+, Z3, and Kani formal verification
>
> [Full comparison →](Fortress-vs-GGRS)
