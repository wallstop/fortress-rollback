//! # Custom Socket Implementation Examples for Fortress Rollback
//!
//! This example demonstrates how to implement the `NonBlockingSocket` trait
//! for custom networking transports. Use this as a guide when integrating
//! with WebSockets, WebRTC, or other networking libraries.
//!
//! ## Key Points
//!
//! 1. **Transport-agnostic**: Fortress Rollback doesn't care how messages are delivered
//! 2. **UDP-like semantics**: Messages should be unordered and unreliable (the library handles retries)
//! 3. **Non-blocking**: `receive_all_messages()` must return immediately, never block
//!
//! ## Included Examples
//!
//! - `ChannelSocket`: In-memory socket using channels (useful for testing)
//! - `WebSocketAdapter`: Skeleton showing how to wrap a WebSocket library
//!
//! Run with: `cargo run --example custom_socket`

// Allow example-specific patterns
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::disallowed_macros,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::type_complexity,
    clippy::use_self
)]

use fortress_rollback::{Message, NonBlockingSocket};
use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};

// =============================================================================
// Example 1: Channel-Based Socket (for local testing)
// =============================================================================

/// A simple address type for our channel-based socket.
/// In a real application, this might be a peer ID, URL, or IP address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChannelPeerId(pub u32);

/// A socket implementation using `std::sync::mpsc` channels.
///
/// This is useful for:
/// - Unit testing without network
/// - Local multiplayer (same process)
/// - Understanding the `NonBlockingSocket` contract
///
/// ## How It Works
///
/// Each `ChannelSocket` has a sender for outgoing messages and a receiver
/// for incoming messages. You create pairs of connected sockets using
/// `ChannelSocket::create_pair()`.
pub struct ChannelSocket {
    /// Our own peer ID
    local_id: ChannelPeerId,
    /// Channel for receiving messages from other peers
    receiver: Receiver<(ChannelPeerId, Message)>,
    /// Senders to other peers, keyed by their peer ID
    peer_senders: Arc<Mutex<Vec<(ChannelPeerId, Sender<(ChannelPeerId, Message)>)>>>,
}

impl ChannelSocket {
    /// Creates a pair of connected sockets for local testing.
    ///
    /// Returns two sockets that can send messages to each other.
    #[must_use]
    pub fn create_pair() -> (Self, Self) {
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();

        let peer1_id = ChannelPeerId(1);
        let peer2_id = ChannelPeerId(2);

        let socket1 = ChannelSocket {
            local_id: peer1_id,
            receiver: rx1,
            peer_senders: Arc::new(Mutex::new(vec![(peer2_id, tx2)])),
        };

        let socket2 = ChannelSocket {
            local_id: peer2_id,
            receiver: rx2,
            peer_senders: Arc::new(Mutex::new(vec![(peer1_id, tx1)])),
        };

        (socket1, socket2)
    }

    /// Returns this socket's peer ID.
    #[allow(dead_code)]
    #[must_use]
    pub fn local_id(&self) -> ChannelPeerId {
        self.local_id
    }
}

impl NonBlockingSocket<ChannelPeerId> for ChannelSocket {
    fn send_to(&mut self, msg: &Message, addr: &ChannelPeerId) {
        let senders = self.peer_senders.lock().unwrap();
        for (peer_id, sender) in senders.iter() {
            if peer_id == addr {
                // Clone the message since we might send to multiple peers
                // In a real network socket, you'd serialize once and send bytes
                let _ = sender.send((self.local_id, msg.clone()));
                return;
            }
        }
        // Silently drop messages to unknown peers (like UDP would)
        println!(
            "Warning: No route to peer {:?} from {:?}",
            addr, self.local_id
        );
    }

    fn receive_all_messages(&mut self) -> Vec<(ChannelPeerId, Message)> {
        let mut messages = Vec::new();

        // Drain all available messages without blocking
        loop {
            match self.receiver.try_recv() {
                Ok(msg) => messages.push(msg),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // Channel closed - peer disconnected
                    break;
                },
            }
        }

        messages
    }
}

// =============================================================================
// Example 2: WebSocket Adapter Skeleton
// =============================================================================

/// A peer identifier for WebSocket connections.
/// In practice, this might be a session ID, user ID, or room slot.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WebSocketPeerId(pub String);

/// Skeleton showing how to wrap a WebSocket library.
///
/// This is a **template** - you'll need to fill in the actual WebSocket
/// implementation based on your chosen library (e.g., `tungstenite`, `tokio-tungstenite`,
/// `ws` for WASM, etc.).
///
/// ## Design Considerations
///
/// 1. **Message framing**: WebSockets are message-oriented, which maps well
///    to Fortress Rollback's `Message` type
///
/// 2. **Binary mode**: Use binary WebSocket messages for efficiency
///
/// 3. **Buffering**: Queue received messages and drain them in `receive_all_messages()`
///
/// 4. **Async handling**: If using async WebSockets, you'll need to poll the
///    WebSocket in your game loop and buffer messages
pub struct WebSocketAdapter {
    /// Queue of messages received but not yet returned
    incoming: VecDeque<(WebSocketPeerId, Message)>,
    /// Placeholder for your WebSocket connection(s)
    /// In practice: HashMap<WebSocketPeerId, WebSocketConnection>
    _connections: (),
}

impl WebSocketAdapter {
    /// Create a new adapter.
    ///
    /// In practice, you'd pass in your WebSocket connection or configuration.
    #[allow(dead_code)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            incoming: VecDeque::new(),
            _connections: (),
        }
    }

    /// Call this from your async runtime or game loop to process WebSocket events.
    ///
    /// This method would:
    /// 1. Poll WebSocket for new messages
    /// 2. Deserialize received binary data into `Message`
    /// 3. Queue messages in `self.incoming`
    #[allow(dead_code)]
    pub fn poll(&mut self) {
        // Example pseudocode:
        //
        // for (peer_id, ws_connection) in &mut self.connections {
        //     while let Some(ws_msg) = ws_connection.try_recv() {
        //         if let Ok(msg) = bincode::deserialize(&ws_msg.into_data()) {
        //             self.incoming.push_back((peer_id.clone(), msg));
        //         }
        //     }
        // }
    }
}

impl Default for WebSocketAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl NonBlockingSocket<WebSocketPeerId> for WebSocketAdapter {
    fn send_to(&mut self, msg: &Message, addr: &WebSocketPeerId) {
        // Serialize the message using bincode (same format Fortress Rollback uses internally)
        let Ok(bytes) = bincode::serde::encode_to_vec(msg, bincode::config::standard()) else {
            eprintln!("Failed to serialize message");
            return;
        };

        // Send via WebSocket (pseudocode)
        println!("Would send {} bytes to peer {:?}", bytes.len(), addr.0);

        // In practice:
        // if let Some(ws) = self.connections.get_mut(addr) {
        //     let _ = ws.send(WebSocketMessage::Binary(bytes));
        // }
    }

    fn receive_all_messages(&mut self) -> Vec<(WebSocketPeerId, Message)> {
        // Drain all queued messages
        self.incoming.drain(..).collect()
    }
}

// =============================================================================
// Example 3: Matchbox Integration Reference
// =============================================================================

/// This module shows the pattern for Matchbox integration.
///
/// Matchbox already implements `NonBlockingSocket` when you enable the `ggrs` feature,
/// so you don't need to write this yourself. This is just for reference.
mod matchbox_reference {
    #![allow(dead_code)]

    /// With Matchbox, the integration is simple:
    ///
    /// ```ignore
    /// use matchbox_socket::WebRtcSocket;
    ///
    /// // Connect to signaling server
    /// let (socket, message_loop) = WebRtcSocket::new_ggrs("wss://matchbox.example.com/room");
    ///
    /// // Spawn message loop (required)
    /// #[cfg(target_arch = "wasm32")]
    /// wasm_bindgen_futures::spawn_local(message_loop);
    /// #[cfg(not(target_arch = "wasm32"))]
    /// std::thread::spawn(move || futures::executor::block_on(message_loop));
    ///
    /// // Wait for peers...
    /// while socket.connected_peers().count() < num_players - 1 {
    ///     // Poll and wait
    /// }
    ///
    /// // Create session - socket already implements NonBlockingSocket!
    /// let session = SessionBuilder::<GameConfig>::new()
    ///     .with_num_players(2).unwrap()
    ///     .add_player(PlayerType::Local, PlayerHandle::new(0))?
    ///     .add_player(PlayerType::Remote(peer_id), PlayerHandle::new(1))?
    ///     .start_p2p_session(socket)?;
    /// ```
    pub struct MatchboxReference;
}

// =============================================================================
// Demo
// =============================================================================

fn main() {
    println!("=== Custom Socket Implementation Examples ===\n");

    demo_channel_socket();
    demo_websocket_adapter();

    println!("\n=== Summary ===");
    println!("To implement NonBlockingSocket for your transport:");
    println!("1. Define an address type (impl Clone + PartialEq + Eq + Ord + Hash + Debug)");
    println!("2. Implement send_to() - serialize and send the Message");
    println!("3. Implement receive_all_messages() - return all pending messages without blocking");
    println!("\nFor browser games, use Matchbox which handles all of this for you!");
}

fn demo_channel_socket() {
    println!("--- Channel Socket Demo ---");
    println!("Creating a pair of connected channel sockets...\n");

    let (socket1, socket2) = ChannelSocket::create_pair();

    // Note: In a real application, you would use these sockets with SessionBuilder:
    //
    // let session = SessionBuilder::<GameConfig>::new()
    //     .with_num_players(2).unwrap()
    //     .add_player(PlayerType::Local, PlayerHandle::new(0))?
    //     .add_player(PlayerType::Remote(ChannelPeerId(2)), PlayerHandle::new(1))?
    //     .start_p2p_session(socket1)?;
    //
    // The session will call send_to() and receive_all_messages() internally.

    println!("Socket 1 ID: {:?}", socket1.local_id());
    println!("Socket 2 ID: {:?}", socket2.local_id());
    println!();
    println!("These sockets are ready to use with SessionBuilder.");
    println!("The session will handle message creation and parsing internally.");
    println!("✓ Channel socket implementation complete!\n");
}

fn demo_websocket_adapter() {
    println!("--- WebSocket Adapter Demo ---");
    println!("The WebSocketAdapter is a skeleton/template.");
    println!("In a real application, you would:");
    println!("  1. Connect to your WebSocket server");
    println!("  2. Implement poll() to receive messages from the WebSocket");
    println!("  3. The session will call send_to() to transmit messages");
    println!();

    let adapter = WebSocketAdapter::new();

    println!(
        "Adapter created: {:?} pending messages",
        adapter.incoming.len()
    );
    println!("\nSee the source code for implementation details.");
}
