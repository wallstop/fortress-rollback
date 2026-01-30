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
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::literal_string_with_formatting_args
)]

use fortress_rollback::{
    Config, FortressError, PlayerHandle, PlayerType, SessionBuilder, UdpNonBlockingSocket,
};
use std::net::SocketAddr;

// Define a minimal config for demonstration
struct GameConfig;

impl Config for GameConfig {
    type Input = u8;
    type State = Vec<u8>;
    type Address = SocketAddr;
}

fn main() {
    println!("=== Fortress Rollback Error Handling Examples ===\n");

    configuration_errors();
    session_setup_errors();
    runtime_error_handling();
    error_recovery_patterns();
}

/// Examples of configuration-time errors and how to handle them
fn configuration_errors() {
    println!("--- Configuration Errors ---\n");

    // Example 1: Invalid FPS
    println!("1. Invalid FPS:");
    let result = SessionBuilder::<GameConfig>::new().with_fps(0);
    match result {
        Ok(_) => println!("   Unexpected success"),
        Err(e) => {
            println!("   Error: {}", e);
            // Pattern match for specific handling
            if let FortressError::InvalidRequest { info } = &e {
                println!("   Reason: {}", info);
            }
        },
    }
    println!();

    // Example 2: Duplicate player handle
    println!("2. Duplicate player handle:");
    let result = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))
        .and_then(|b| b.add_player(PlayerType::Local, PlayerHandle::new(0))); // Same handle!
    match result {
        Ok(_) => println!("   Unexpected success"),
        Err(FortressError::InvalidRequest { info }) => {
            println!("   Error: Player handle already in use");
            println!("   Details: {}", info);
        },
        Err(e) => println!("   Unexpected error type: {}", e),
    }
    println!();

    // Example 3: Invalid player handle for player type
    println!("3. Invalid handle for player type:");
    let result = SessionBuilder::<GameConfig>::new()
        .with_num_players(2).unwrap()
        // Handle 5 is invalid for a local player in a 2-player game
        .add_player(PlayerType::Local, PlayerHandle::new(5));
    match result {
        Ok(_) => println!("   Unexpected success"),
        Err(FortressError::InvalidRequest { info }) => {
            println!("   Error: Invalid player handle");
            println!("   Details: {}", info);
        },
        Err(e) => println!("   Unexpected error type: {}", e),
    }
    println!();

    // Example 4: Invalid max_frames_behind
    println!("4. Invalid max_frames_behind:");
    let result = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .unwrap()
        .with_max_frames_behind(0); // Must be >= 1
    match result {
        Ok(_) => println!("   Unexpected success"),
        Err(FortressError::InvalidRequest { info }) => {
            println!("   Error: Invalid configuration");
            println!("   Details: {}", info);
        },
        Err(e) => println!("   Unexpected error type: {}", e),
    }
    println!();
}

/// Examples of session startup errors
fn session_setup_errors() {
    println!("--- Session Setup Errors ---\n");

    // Example 1: Socket binding error
    println!("1. Socket binding error:");
    // Try to bind to a privileged port (will fail without root)
    let socket_result = UdpNonBlockingSocket::bind_to_port(80);
    match socket_result {
        Ok(_) => println!("   Bound to port 80 (running as root?)"),
        Err(e) => {
            println!("   Error: {}", e);
            println!("   Recovery: Try a different port or check permissions");
        },
    }
    println!();

    // Example 2: Starting session without enough players
    println!("2. Starting P2P session without players:");
    // Create a socket first (on a high port that should work)
    if let Ok(socket) = UdpNonBlockingSocket::bind_to_port(0) {
        let builder = SessionBuilder::<GameConfig>::new()
            .with_num_players(2)
            .unwrap();
        // Don't add any players!

        let result = builder.start_p2p_session(socket);
        match result {
            Ok(_) => println!("   Unexpected success"),
            Err(FortressError::InvalidRequest { info }) => {
                println!("   Error: Cannot start session");
                println!("   Details: {}", info);
            },
            Err(e) => println!("   Error: {}", e),
        }
    }
    println!();
}

/// Examples of runtime error handling during gameplay
fn runtime_error_handling() {
    println!("--- Runtime Error Handling ---\n");

    // Create a simple 2-player setup
    println!("Setting up a test session...");

    // Use fixed ports for the example
    let port1: u16 = 17000;
    let port2: u16 = 17001;

    let socket1 = match UdpNonBlockingSocket::bind_to_port(port1) {
        Ok(s) => s,
        Err(e) => {
            println!("   Failed to create socket on port {}: {}", port1, e);
            println!("   Port may be in use. Try again later.");
            return;
        },
    };

    let socket2 = match UdpNonBlockingSocket::bind_to_port(port2) {
        Ok(s) => s,
        Err(e) => {
            println!("   Failed to create socket on port {}: {}", port2, e);
            println!("   Port may be in use. Try again later.");
            return;
        },
    };

    let addr1: SocketAddr = format!("127.0.0.1:{}", port1).parse().unwrap();
    let addr2: SocketAddr = format!("127.0.0.1:{}", port2).parse().unwrap();

    // Create sessions
    let sess1 = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .unwrap()
        .add_player(PlayerType::Local, PlayerHandle::new(0))
        .and_then(|b| b.add_player(PlayerType::Remote(addr2), PlayerHandle::new(1)))
        .and_then(|b| b.start_p2p_session(socket1));

    let sess2 = SessionBuilder::<GameConfig>::new()
        .with_num_players(2)
        .unwrap()
        .add_player(PlayerType::Remote(addr1), PlayerHandle::new(0))
        .and_then(|b| b.add_player(PlayerType::Local, PlayerHandle::new(1)))
        .and_then(|b| b.start_p2p_session(socket2));

    let (mut sess1, mut sess2) = match (sess1, sess2) {
        (Ok(s1), Ok(s2)) => (s1, s2),
        (Err(e), _) | (_, Err(e)) => {
            println!("   Failed to create sessions: {}", e);
            return;
        },
    };

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
        Err(e) => println!("   Error: {}", e),
    }

    // Example 2: Getting network stats before sync
    println!("\n2. Getting network stats before sync:");
    let stats_result = sess1.network_stats(PlayerHandle::new(1));
    match stats_result {
        Ok(stats) => println!("   Stats: {:?}", stats),
        Err(FortressError::NotSynchronized) => {
            println!("   Error: Cannot get stats - not synchronized");
            println!("   Recovery: Wait for synchronization to complete");
        },
        Err(e) => println!("   Error: {}", e),
    }

    // Example 3: Invalid player handle
    println!("\n3. Invalid player handle:");
    let result = sess1.add_local_input(PlayerHandle::new(99), 42u8);
    match result {
        Ok(()) => println!("   Unexpected success"),
        Err(FortressError::InvalidPlayerHandle { handle, max_handle }) => {
            println!(
                "   Error: Invalid handle {} (max valid: {})",
                handle, max_handle
            );
            println!("   Recovery: Use handles 0 to num_players-1");
        },
        Err(FortressError::InvalidRequest { info }) => {
            println!("   Error: {}", info);
        },
        Err(e) => println!("   Error: {}", e),
    }

    // Do some polling to let them sync
    for _ in 0..10 {
        sess1.poll_remote_clients();
        sess2.poll_remote_clients();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    println!();
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
