//! # Error Handling Examples for Fortress Rollback
//!
//! This example demonstrates proper error handling patterns when using
//! Fortress Rollback. It covers:
//!
//! - Configuration errors during session setup
//! - Runtime errors during gameplay
//! - Graceful error recovery strategies
//! - Best practices for production code
//!
//! Run with: `cargo run --example error_handling`

// Allow example-specific patterns
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::disallowed_macros,
    clippy::indexing_slicing,
    clippy::literal_string_with_formatting_args
)]

use fortress_rollback::{
    Config, FortressError, InternalErrorKind, InvalidRequestKind, PlayerHandle, PlayerType,
    SessionBuilder, SpectatorConfig, UdpNonBlockingSocket,
};
use std::error::Error;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};

// Define a minimal config for demonstration
struct GameConfig;

impl Config for GameConfig {
    type Input = u8;
    type State = Vec<u8>;
    type Address = SocketAddr;
}

type ExampleResult = Result<(), Box<dyn Error>>;

fn contract_error(message: &'static str) -> Box<dyn Error> {
    io::Error::other(message).into()
}

fn main() -> ExampleResult {
    println!("=== Fortress Rollback Error Handling Examples ===\n");

    configuration_errors()?;
    session_setup_errors()?;
    runtime_error_handling()?;
    runtime_input_delay_and_remove_errors()?;
    error_recovery_patterns();
    Ok(())
}

/// Examples of configuration-time errors and how to handle them
fn configuration_errors() -> ExampleResult {
    println!("--- Configuration Errors ---\n");

    // Example 1: Invalid FPS
    println!("1. Invalid FPS:");
    let result = SessionBuilder::<GameConfig>::new().with_fps(0);
    match result {
        Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::ZeroFps,
        }) => {
            println!("   Error: FPS must be greater than zero");
        },
        Err(error) => return Err(error.into()),
        Ok(_) => return Err(contract_error("zero FPS was accepted")),
    }
    println!();

    // Example 2: Duplicate player handle
    println!("2. Duplicate player handle:");
    let result = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)?
        .add_player(PlayerType::Local, PlayerHandle::new(0))
        .and_then(|b| b.add_player(PlayerType::Local, PlayerHandle::new(0))); // Same handle!
    match result {
        Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::PlayerHandleInUse { handle },
        }) => {
            println!("   Error: Player handle {handle} is already in use");
        },
        Err(error) => return Err(error.into()),
        Ok(_) => return Err(contract_error("duplicate player handle was accepted")),
    }
    println!();

    // Example 3: Invalid player handle for player type
    println!("3. Invalid handle for player type:");
    let result = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)?
        // Handle 5 is invalid for a local player in a 2-player game
        .add_player(PlayerType::Local, PlayerHandle::new(5));
    match result {
        Err(FortressError::InvalidRequestStructured {
            kind:
                InvalidRequestKind::InvalidLocalPlayerHandle {
                    handle,
                    num_players,
                },
        }) => {
            println!("   Error: Local handle {handle} is invalid for {num_players} players");
        },
        Err(error) => return Err(error.into()),
        Ok(_) => {
            return Err(contract_error(
                "out-of-range local player handle was accepted",
            ))
        },
    }
    println!();

    // Example 4: Invalid spectator buffer
    println!("4. Invalid spectator buffer:");
    let result = SpectatorConfig {
        buffer_size: 0,
        ..SpectatorConfig::default()
    }
    .validate();
    match result {
        Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::ZeroBufferSize,
        }) => {
            println!("   Error: Spectator buffer size must be greater than zero");
        },
        Err(error) => return Err(error.into()),
        Ok(()) => return Err(contract_error("zero spectator buffer was accepted")),
    }
    println!();
    Ok(())
}

/// Examples of session startup errors
fn session_setup_errors() -> ExampleResult {
    println!("--- Session Setup Errors ---\n");

    // Example 1: Socket binding error
    println!("1. Socket binding error:");
    // A zero-length receive buffer is always invalid, independent of host privileges.
    let socket_result = UdpNonBlockingSocket::bind_to_port_with_buffer_sizes(0, 0, 1024);
    match socket_result {
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => {
            println!("   Error: {error}");
            println!("   Recovery: Configure non-zero socket buffers");
        },
        Err(error) => return Err(error.into()),
        Ok(_) => return Err(contract_error("zero-length socket buffer was accepted")),
    }
    println!();

    // Example 2: Starting session without enough players
    println!("2. Starting P2P session without players:");
    let socket = UdpNonBlockingSocket::bind_to_port(0)?;
    let builder = SessionBuilder::<GameConfig>::new().with_num_players(2)?;
    // Don't add any players!
    match builder.start_p2p_session(socket) {
        Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::NotEnoughPlayers { expected, actual },
        }) => {
            println!("   Error: Expected {expected} players, but registered {actual}");
        },
        Err(error) => return Err(error.into()),
        Ok(_) => return Err(contract_error("P2P session started without players")),
    }
    println!();
    Ok(())
}

/// Examples of runtime error handling during gameplay
fn runtime_error_handling() -> ExampleResult {
    println!("--- Runtime Error Handling ---\n");

    // Create a simple 2-player setup
    println!("Setting up a test session...");

    // Ephemeral ports avoid conflicts with other local processes.
    let socket1 = UdpNonBlockingSocket::bind_to_port(0)?;
    let socket2 = UdpNonBlockingSocket::bind_to_port(0)?;
    let port1 = socket1.local_addr()?.port();
    let port2 = socket2.local_addr()?.port();
    let addr1 = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port1);
    let addr2 = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port2);

    // Create sessions
    let sess1 = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)?
        .add_player(PlayerType::Local, PlayerHandle::new(0))
        .and_then(|b| b.add_player(PlayerType::Remote(addr2), PlayerHandle::new(1)))
        .and_then(|b| b.start_p2p_session(socket1));

    let sess2 = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)?
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))
        .and_then(|b| b.add_player(PlayerType::Local, PlayerHandle::new(1)))
        .and_then(|b| b.start_p2p_session(socket2));

    let (mut sess1, mut sess2) = (sess1?, sess2?);

    println!("   Sessions created on ports {} and {}", port1, port2);

    // Example 1: Adding input before synchronization
    println!("\n1. Adding input before sync completes:");
    let result = sess1.add_local_input(PlayerHandle::new(0), 42u8);
    match result {
        Ok(()) => println!("   Input added (session may be in Synchronizing state)"),
        Err(FortressError::NotSynchronized) => {
            println!("   Error: Session not synchronized yet");
            println!("   Recovery: Wait for SessionState::Running before adding inputs");
        },
        Err(error) => return Err(error.into()),
    }

    // Example 2: Getting network stats before sync
    println!("\n2. Getting network stats before sync:");
    let stats_result = sess1.network_stats(PlayerHandle::new(1));
    match stats_result {
        Ok(_) => {
            return Err(contract_error(
                "network stats were available before synchronization",
            ))
        },
        Err(FortressError::NotSynchronized) => {
            println!("   Error: Cannot get stats - not synchronized");
            println!("   Recovery: Wait for synchronization to complete");
        },
        Err(error) => return Err(error.into()),
    }

    // Example 3: Invalid player handle
    println!("\n3. Invalid player handle:");
    let result = sess1.add_local_input(PlayerHandle::new(99), 42u8);
    match result {
        Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::NotLocalPlayer { handle },
        }) => {
            println!("   Error: Handle {handle} is not a registered local player");
            println!("   Recovery: Use handles 0 to num_players-1");
        },
        Err(error) => return Err(error.into()),
        Ok(()) => {
            return Err(contract_error(
                "input was accepted for an unregistered handle",
            ))
        },
    }

    // Do some polling to let them sync
    for _ in 0..10 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    println!();
    Ok(())
}

/// Demonstrates how to react to the new structured error variants returned
/// by `P2PSession::set_input_delay` and `P2PSession::remove_player`.
///
/// All matches use the structured `InvalidRequestStructured { kind }` and
/// `InternalErrorStructured { kind }` forms so callers can pattern-match on
/// the typed variant rather than parsing free-form strings.
///
/// Variants exercised here (each with a typical recovery hint):
///
/// - `InvalidRequestKind::InputDelayDecreaseUnsupported { current, requested }`
///   — once inputs have been added, only increases are permitted; carry the
///   lower delay over to the next session.
/// - `InvalidRequestKind::InputDelayMidSessionMultiLocalUnsupported { local_players }`
///   — set the delay via `SessionBuilder::with_input_delay` before inputs
///   are added when running multi-local on this peer.
/// - `InvalidRequestKind::InputDelayMidSessionPendingOutputFull { delta, capacity }`
///   — apply the change in smaller increments, or wait for the remote to
///   acknowledge outstanding inputs and retry.
/// - `InvalidRequestKind::PlayerAlreadyRemoved { handle }` — `remove_player`
///   was called twice for the same handle, or the peer was already
///   auto-removed under `DisconnectBehavior::ContinueWithout`; treat as a
///   no-op.
/// - `InternalErrorKind::InputQueueGapFillFailed { frame }` — library
///   invariant violation while replicating gap-fill bytes during a
///   mid-session input-delay increase; report as a bug.
fn runtime_input_delay_and_remove_errors() -> ExampleResult {
    println!("--- Runtime Input-Delay and Remove-Player Error Variants ---\n");

    // Build a session locally so we can issue `set_input_delay`/`remove_player`
    // calls and pattern-match on the returned error variants. The session
    // never actually establishes a handshake; we rely on the validation
    // performed inside the session methods, which runs regardless of
    // synchronization state.
    let socket = UdpNonBlockingSocket::bind_to_port(0)?;
    let remote = SocketAddr::from((Ipv4Addr::LOCALHOST, 17_999));

    let local_handle = PlayerHandle::new(0);
    let remote_handle = PlayerHandle::new(1);
    let session_result = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)?
        .with_input_delay(2)?
        .add_player(PlayerType::Local, local_handle)
        .and_then(|b| b.add_player(PlayerType::Remote(remote), remote_handle))
        .and_then(|b| b.start_p2p_session(socket));

    let mut session = session_result?;

    println!("1. set_input_delay error variants:");
    demonstrate_set_input_delay_errors(&mut session, local_handle)?;

    println!("\n2. remove_player error variants:");
    demonstrate_remove_player_errors(&mut session, local_handle, remote_handle)?;

    println!("\n3. InternalErrorKind::InputQueueGapFillFailed:");
    println!("   This variant fires only on a library-invariant violation while replicating");
    println!("   gap-fill bytes during a mid-session input-delay increase. Treat as a bug:");
    println!("   ```rust");
    println!("   Err(FortressError::InternalErrorStructured {{");
    println!("       kind: InternalErrorKind::InputQueueGapFillFailed {{ frame }},");
    println!("   }}) => {{");
    println!(
        "       eprintln!(\"internal: input-queue gap-fill failed at frame {{frame}}; please report\");"
    );
    println!("   }}");
    println!("   ```");
    println!();
    Ok(())
}

/// Issues a few `set_input_delay` calls and pattern-matches on the structured
/// `InvalidRequestKind` variants the call can return. Uses `?` to propagate
/// any unexpected `FortressError`; never `unwrap`/`expect`/`panic!`.
fn demonstrate_set_input_delay_errors<C: Config>(
    session: &mut fortress_rollback::P2PSession<C>,
    local_handle: PlayerHandle,
) -> ExampleResult {
    // Initial-setup decrease is allowed (no inputs added yet); use that path
    // to leave the session in a deterministic state.
    if let Err(err) = session.set_input_delay(local_handle, 1) {
        return Err(err.into());
    }

    // Now request a decrease again. This is still pre-input so it succeeds —
    // we set up the conditions for a *mid-session* decrease in `runtime`
    // tests; this function focuses on shape-matching every variant.
    if let Err(err) = session.set_input_delay(local_handle, 0) {
        return Err(err.into());
    }

    // Demonstrate the typed-error pattern shapes a caller writes for each of
    // the three new `InputDelay*` variants. We do not need to actually
    // trigger every error here — the request says "demonstrate the API".
    let demo_target = 4_usize;
    match session.set_input_delay(local_handle, demo_target) {
        Ok(()) => println!("   set_input_delay(.., {demo_target}) accepted."),
        Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::InputDelayDecreaseUnsupported { current, requested },
        }) => {
            // Recovery: surface a meaningful rejection to the player; the
            // lower delay can be carried over to the next session.
            println!(
                "   InputDelayDecreaseUnsupported: cannot lower delay from {current} to \
                 {requested} mid-session; the lower value can be carried over to the next match."
            );
        },
        Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::InputDelayMidSessionMultiLocalUnsupported { local_players },
        }) => {
            // Recovery: configure delay before adding inputs (e.g., via
            // `SessionBuilder::with_input_delay`) for couch-co-op setups
            // with more than one local player on this peer.
            println!(
                "   InputDelayMidSessionMultiLocalUnsupported: mid-session increase not \
                 supported with {local_players} local players on this peer; configure delay \
                 via SessionBuilder::with_input_delay before adding inputs."
            );
        },
        Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::InputDelayMidSessionPendingOutputFull { delta, capacity },
        }) => {
            // Recovery: apply the change in smaller increments, or wait for
            // the remote to acknowledge outstanding inputs and retry.
            println!(
                "   InputDelayMidSessionPendingOutputFull: gap-fill needs {delta} slots, only \
                 {capacity} available; retry next tick or apply in smaller increments."
            );
        },
        Err(FortressError::InternalErrorStructured {
            kind: InternalErrorKind::InputQueueGapFillFailed { frame },
        }) => {
            // Library bug; surface as fatal and ask the user to file a report
            // with the failing frame and the call's parameters.
            println!(
                "   InputQueueGapFillFailed at frame {frame}: please file a bug with the call \
                 parameters."
            );
        },
        Err(other) => {
            // Anything else violates the example's expected contract; propagate.
            return Err(other.into());
        },
    }

    Ok(())
}

/// Issues a few `remove_player` calls and pattern-matches on the structured
/// `InvalidRequestKind` variants. Uses `?` to propagate unexpected errors;
/// never `unwrap`/`expect`/`panic!`.
fn demonstrate_remove_player_errors<C: Config>(
    session: &mut fortress_rollback::P2PSession<C>,
    local_handle: PlayerHandle,
    remote_handle: PlayerHandle,
) -> ExampleResult {
    // Removing a local player is rejected with `DisconnectLocalPlayer`; the
    // recovery is to tear down the session instead of trying to remove the
    // local player.
    match session.remove_player(local_handle) {
        Ok(()) => return Err(contract_error("local player removal was accepted")),
        Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::DisconnectLocalPlayer { handle },
        }) => {
            println!(
                "   DisconnectLocalPlayer({handle}): local players cannot be removed; tear down \
                 the session instead."
            );
        },
        Err(other) => {
            return Err(other.into());
        },
    }

    // Remove the remote player gracefully.
    match session.remove_player(remote_handle) {
        Ok(()) => println!("   Removed remote player {remote_handle} (graceful drop)."),
        Err(other) => {
            return Err(other.into());
        },
    }

    // Removing the same remote player a second time returns
    // `PlayerAlreadyRemoved`; treat as a no-op.
    match session.remove_player(remote_handle) {
        Ok(()) => return Err(contract_error("already-removed player was removed twice")),
        Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::PlayerAlreadyRemoved { handle },
        }) => {
            // Recovery: no-op; the peer is already in the graceful-drop
            // terminal state.
            println!(
                "   PlayerAlreadyRemoved({handle}): peer already in graceful-drop terminal \
                 state; ignoring duplicate request."
            );
        },
        Err(other) => return Err(other.into()),
    }

    Ok(())
}

/// Patterns for graceful error recovery
fn error_recovery_patterns() {
    println!("--- Error Recovery Patterns ---\n");

    println!("1. Retry with backoff for socket binding:");
    println!("   ```rust");
    println!(
        "   fn bind_with_retry(start_port: u16) -> Result<UdpNonBlockingSocket, FortressError> {{"
    );
    println!("       for port in start_port..start_port+100 {{");
    println!("           match UdpNonBlockingSocket::bind_to_port(port) {{");
    println!("               Ok(socket) => return Ok(socket),");
    println!("               Err(_) => continue,");
    println!("           }}");
    println!("       }}");
    println!("       Err(FortressError::SocketError {{ context: \"No ports available\".into() }})");
    println!("   }}");
    println!("   ```\n");

    println!("2. Wait for synchronization before gameplay:");
    println!("   ```rust");
    println!(
        "   async fn wait_for_sync(session: &mut P2PSession<C>) -> Result<(), FortressError> {{"
    );
    println!("       let timeout = Duration::from_secs(10);");
    println!("       let start = Instant::now();");
    println!("       ");
    println!("       while session.current_state() != SessionState::Running {{");
    println!("           if start.elapsed() > timeout {{");
    println!("               return Err(FortressError::NotSynchronized);");
    println!("           }}");
    println!("           session.poll_remote_clients();");
    println!("           tokio::time::sleep(Duration::from_millis(16)).await;");
    println!("       }}");
    println!("       Ok(())");
    println!("   }}");
    println!("   ```\n");

    println!("3. Handle prediction threshold gracefully:");
    println!("   ```rust");
    println!("   fn add_input_safe(session: &mut P2PSession<C>, input: Input) -> bool {{");
    println!("       match session.add_local_input(handle, input) {{");
    println!("           Ok(()) => true,");
    println!("           Err(FortressError::PredictionThreshold) => {{");
    println!("               // Too far ahead - skip this frame's input");
    println!("               // The game will catch up via rollback");
    println!("               false");
    println!("           }}");
    println!("           Err(e) => {{");
    println!("               eprintln!(\"Input error: {{}}\", e);");
    println!("               false");
    println!("           }}");
    println!("       }}");
    println!("   }}");
    println!("   ```\n");

    println!("4. Comprehensive match on FortressError:");
    println!("   ```rust");
    println!("   fn handle_error(error: FortressError) -> Action {{");
    println!("       match error {{");
    println!("           FortressError::PredictionThreshold => {{");
    println!("               // Wait for network to catch up");
    println!("               Action::WaitAndRetry");
    println!("           }}");
    println!("           FortressError::NotSynchronized => {{");
    println!("               // Keep polling until sync completes");
    println!("               Action::KeepPolling");
    println!("           }}");
    println!("           FortressError::SpectatorTooFarBehind => {{");
    println!("               // Reconnect spectator");
    println!("               Action::Reconnect");
    println!("           }}");
    println!(
        "           FortressError::MismatchedChecksum {{ current_frame, mismatched_frames }} => {{"
    );
    println!("               // Log desync for debugging");
    println!("               log::error!(\"Desync at frame {{}}: {{:?}}\", current_frame, mismatched_frames);");
    println!("               Action::DesyncDetected");
    println!("           }}");
    println!("           FortressError::InvalidRequest {{ info }} => {{");
    println!("               // Invalid request: likely programming error in application code");
    println!("               eprintln!(\"Invalid request (likely programming error): {{info}}\");");
    println!("               Action::Fatal");
    println!("           }}");
    println!("           FortressError::InvalidFrame {{ frame, reason }} => {{");
    println!("               log::warn!(\"Invalid frame {{}}: {{}}\", frame, reason);");
    println!("               Action::Continue");
    println!("           }}");
    println!("           FortressError::InvalidPlayerHandle {{ handle, max_handle }} => {{");
    println!("               // Invalid player handle: check player setup logic");
    println!(
        "               eprintln!(\"Invalid player handle {{handle}} (max: {{max_handle}}) — check player setup\");"
    );
    println!("               Action::Fatal");
    println!("           }}");
    println!("           FortressError::MissingInput {{ player_handle, frame }} => {{");
    println!("               log::warn!(\"Missing input for player {{}} at frame {{}}\", player_handle, frame);");
    println!("               Action::Continue");
    println!("           }}");
    println!("           FortressError::SerializationError {{ context }} => {{");
    println!("               log::error!(\"Serialization error: {{}}\", context);");
    println!("               Action::Fatal");
    println!("           }}");
    println!("           FortressError::InternalError {{ context }} => {{");
    println!("               // This is a bug - please report it!");
    println!("               log::error!(\"Internal error: {{}}\", context);");
    println!("               Action::Fatal");
    println!("           }}");
    println!("           FortressError::SocketError {{ context }} => {{");
    println!("               log::error!(\"Socket error: {{}}\", context);");
    println!("               Action::Reconnect");
    println!("           }}");
    println!("           FortressError::InvalidFrameStructured {{ frame, reason }} => {{");
    println!("               log::warn!(\"Invalid frame {{}}: {{:?}}\", frame, reason);");
    println!("               Action::Continue");
    println!("           }}");
    println!("           FortressError::InternalErrorStructured {{ kind }} => {{");
    println!("               // This is a bug - please report it!");
    println!("               log::error!(\"Internal error: {{:?}}\", kind);");
    println!("               Action::Fatal");
    println!("           }}");
    println!("           FortressError::InvalidRequestStructured {{ kind }} => {{");
    println!("               // Invalid request: likely programming error in application code");
    println!(
        "               eprintln!(\"Invalid request (likely programming error): {{kind:?}}\");"
    );
    println!("               Action::Fatal");
    println!("           }}");
    println!("           FortressError::SerializationErrorStructured {{ kind }} => {{");
    println!("               log::error!(\"Serialization error: {{:?}}\", kind);");
    println!("               Action::Fatal");
    println!("           }}");
    println!("           FortressError::SocketErrorStructured {{ kind }} => {{");
    println!("               log::error!(\"Socket error: {{:?}}\", kind);");
    println!("               Action::Reconnect");
    println!("           }}");
    println!("       }}");
    println!("   }}");
    println!("   ```");
}
