---
title: Fortress Rollback
description: A correctness-first fork of GGRS providing peer-to-peer rollback networking for deterministic multiplayer games in Rust.
---

<p align="center">
  <img src="../assets/logo.svg" alt="Fortress Rollback" width="160">
</p>

# Fortress Rollback

## *Deterministic Rollback Netcode Built on Correctness*

Fortress Rollback is a **correctness-first** Rust library for peer-to-peer rollback networking in deterministic multiplayer games. Built as a hardened fork of GGRS (Good Game Rollback System), it prioritizes safety, formal verification, and production reliability.

---

## :material-star-four-points: Key Features

<!-- markdownlint-disable MD030 -->
<div class="grid cards" markdown>

-   :material-shield-check:{ .lg .middle } **Zero-Panic Guarantee**

    ---

    100% safe Rust with no panics in production code. All operations return proper `Result` types—your game server won't crash from unexpected states.

    [:octicons-arrow-right-24: Learn about error handling](user-guide.md#handling-requests)

-   :material-sync:{ .lg .middle } **Rollback Netcode**

    ---

    Peer-to-peer architecture with input prediction and rollback. Hides latency by predicting inputs and seamlessly correcting when actual inputs arrive.

    [:octicons-arrow-right-24: Read the architecture](architecture.md)

-   :material-check-decagram:{ .lg .middle } **Formally Verified**

    ---

    Critical paths verified with TLA+ model checking, Z3 SMT proofs, and Kani for Rust. Protocol correctness proven, not just tested.

    [:octicons-arrow-right-24: View specifications](specs/formal-spec.md)

-   :material-equal-box:{ .lg .middle } **Deterministic by Design**

    ---

    Same inputs = same outputs, guaranteed. Deterministic data structures, hashing, and RNG throughout. No hidden non-determinism.

    [:octicons-arrow-right-24: Determinism model](specs/determinism-model.md)

</div>
<!-- markdownlint-enable MD030 -->

---

## :material-rocket-launch: Quick Start

Get up and running with Fortress Rollback in minutes.

<!-- markdownlint-disable MD046 -->
=== "Cargo.toml"

    ```toml
    [dependencies]
    fortress_rollback = "0.11"
    serde = { version = "1.0", features = ["derive"] }
    ```

=== "Basic Session"

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
<!-- markdownlint-enable MD046 -->

---

## :material-map-marker-path: Where to Go Next

<!-- markdownlint-disable MD030 -->
<div class="grid cards" markdown>

-   :material-book-open-variant:{ .lg .middle } **User Guide**

    ---

    Complete walkthrough of sessions, inputs, state management, and network events.

    [:octicons-arrow-right-24: Start learning](user-guide.md)

-   :material-api:{ .lg .middle } **API Documentation**

    ---

    Full API reference with types, traits, and function signatures.

    [:octicons-arrow-right-24: docs.rs](https://docs.rs/fortress-rollback)

-   :material-sitemap:{ .lg .middle } **Architecture**

    ---

    Deep dive into internal architecture, data flow, and protocol design.

    [:octicons-arrow-right-24: Explore](architecture.md)

-   :material-source-pull:{ .lg .middle } **Contributing**

    ---

    Guidelines for contributors, including our zero-panic policy and testing requirements.

    [:octicons-arrow-right-24: Contribute](contributing.md)

</div>
<!-- markdownlint-enable MD030 -->

---

<!-- markdownlint-disable MD046 -->
!!! info "Fork of GGRS"

    Fortress Rollback is a hardened fork of [GGRS](https://github.com/gschup/ggrs) (Good Game Rollback System). It maintains API compatibility where possible while adding formal verification, eliminating panics, and fixing determinism bugs.

    **Key improvements over GGRS:**

    - :material-shield-check: All `panic!` and `assert!` converted to recoverable errors
    - :material-sort-variant: Deterministic `BTreeMap`/`BTreeSet` instead of `HashMap`/`HashSet`
    - :material-test-tube: ~1500 tests with ~92% code coverage
    - :material-file-certificate: TLA+, Z3, and Kani formal verification

    [:octicons-arrow-right-24: Full comparison](fortress-vs-ggrs.md)
<!-- markdownlint-enable MD046 -->
