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
//! 4. **Bounded receive batches**: return a capped batch each poll and leave excess queued
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

use fortress_rollback::{network::codec, Message, NonBlockingSocket};
use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};

const MAX_WEBSOCKET_MESSAGES_PER_POLL: usize = 64;
const MAX_CHANNEL_MESSAGES_PER_POLL: usize = 64;

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
        if messages
            .try_reserve_exact(MAX_CHANNEL_MESSAGES_PER_POLL)
            .is_err()
        {
            return messages;
        }

        // Drain a bounded batch without blocking; excess remains queued.
        while messages.len() < MAX_CHANNEL_MESSAGES_PER_POLL {
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
/// implementation based on your chosen library (e.g., `tungstenite`,
/// `tokio-tungstenite`, or a browser-capable WebSocket crate under
/// `wasm32-unknown-unknown`).
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
        // let mut attempts = 0;
        // for (peer_id, ws_connection) in &mut self.connections {
        //     while attempts < MAX_WEBSOCKET_MESSAGES_PER_POLL
        //         && self.incoming.len() < MAX_WEBSOCKET_MESSAGES_PER_POLL
        //     {
        //         let Some(ws_msg) = ws_connection.try_recv() else { break };
        //         attempts += 1;
        //         if let Ok((msg, _consumed)) = codec::decode_message(&ws_msg.into_data()) {
        //             self.incoming.push_back((peer_id.clone(), msg));
        //         }
        //     }
        //     if attempts >= MAX_WEBSOCKET_MESSAGES_PER_POLL
        //         || self.incoming.len() >= MAX_WEBSOCKET_MESSAGES_PER_POLL
        //     {
        //         break;
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
        // Serialize the message using Fortress Rollback's deterministic codec.
        let Ok(bytes) = codec::encode(msg) else {
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
        // Drain a bounded batch and leave any excess queued for the next poll.
        let batch_len = self.incoming.len().min(MAX_WEBSOCKET_MESSAGES_PER_POLL);
        self.incoming.drain(..batch_len).collect()
    }
}

// =============================================================================
// Example 3: Matchbox Adapter Reference
// =============================================================================

/// This module outlines the adapter boundary for Matchbox 0.14.
///
/// Matchbox's `ggrs` feature implements the upstream GGRS trait for
/// `WebRtcChannel`; it does not implement Fortress Rollback's trait. Browser
/// applications should leave that feature disabled, take a raw channel from the
/// socket, and wrap it in their own bounded `NonBlockingSocket` implementation.
mod matchbox_reference {
    #![allow(dead_code)]

    /// A browser-only Matchbox integration has this shape:
    ///
    /// ```ignore
    /// use matchbox_socket::WebRtcSocket;
    ///
    /// // Connect to the signaling server with one unreliable data channel.
    /// let (mut socket, message_loop) =
    ///     WebRtcSocket::new_unreliable("wss://matchbox.example.com/room");
    ///
    /// // The browser executor must keep the Matchbox message loop running.
    /// wasm_bindgen_futures::spawn_local(async move {
    ///     if let Err(error) = message_loop.await {
    ///         report_matchbox_error(error); // application logger/error handler
    ///     }
    /// });
    ///
    /// // Split the channel so the adapter can poll a fixed maximum from the
    /// // receiver per call, leaving excess packets queued for later. Calling
    /// // channel.receive() would drain Matchbox's entire queue into a new Vec.
    /// let channel = socket.take_channel(0)?;
    /// let (sender, receiver) = channel.split();
    /// let fortress_socket = FortressMatchboxAdapter::new(sender, receiver);
    ///
    /// let session = SessionBuilder::<GameConfig>::new()
    ///     .with_num_players(2)?
    ///     .add_player(PlayerType::Local, PlayerHandle::new(0))?
    ///     .add_player(PlayerType::Remote(peer_id), PlayerHandle::new(1))?
    ///     .start_p2p_session(fortress_socket)?;
    /// ```
    ///
    /// Declare Matchbox only under
    /// `cfg(all(target_arch = "wasm32", target_os = "unknown"))`. Godot Web
    /// GDExtensions target Emscripten and need an engine/Godot transport adapter
    /// with no wasm-bindgen-family dependencies in their normal graph.
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
    println!("3. Implement receive_all_messages() - return a bounded batch without blocking");
    println!("\nFor browser games, adapt a raw Matchbox channel with the same codec pattern.");
}

fn demo_channel_socket() {
    println!("--- Channel Socket Demo ---");
    println!("Creating a pair of connected channel sockets...\n");

    let (socket1, socket2) = ChannelSocket::create_pair();

    // Note: In a real application, you would use these sockets with SessionBuilder:
    //
    // let session = SessionBuilder::<GameConfig>::new()
    //     .with_num_players(2)?
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
