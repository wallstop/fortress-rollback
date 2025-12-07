//! A configurable socket wrapper for network fault injection testing.
//!
//! [`ChaosSocket`] wraps any [`NonBlockingSocket`] implementation to simulate
//! adverse network conditions including latency, jitter, packet loss,
//! reordering, and duplication. This is essential for testing rollback
//! networking behavior under realistic network conditions.
//!
//! # Example
//!
//! ```rust,no_run
//! use fortress_rollback::{ChaosSocket, ChaosConfig, NonBlockingSocket};
//! use std::net::SocketAddr;
//! use std::time::Duration;
//!
//! // Configure network chaos
//! let config = ChaosConfig::builder()
//!     .latency(Duration::from_millis(50))
//!     .jitter(Duration::from_millis(20))
//!     .packet_loss_rate(0.05)  // 5% packet loss
//!     .seed(42)  // Deterministic for testing
//!     .build();
//!
//! // Wrap an existing socket
//! // let chaos_socket = ChaosSocket::new(inner_socket, config);
//! ```
//!
//! # Features
//!
//! - **Latency**: Constant delay added to all packets
//! - **Jitter**: Random variation in latency (uniform distribution)
//! - **Packet Loss**: Configurable drop rate for outgoing/incoming packets
//! - **Duplication**: Randomly duplicate packets
//! - **Reordering**: Shuffle packet delivery order
//! - **Asymmetric Conditions**: Different settings for send vs receive
//! - **Deterministic**: Seeded RNG for reproducible test scenarios

use std::collections::VecDeque;
use std::hash::Hash;
use std::time::{Duration, Instant};

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::network::messages::Message;
use crate::NonBlockingSocket;

/// Configuration for network chaos simulation.
///
/// Use [`ChaosConfig::builder()`] for a fluent configuration API.
/// All durations default to zero and all rates default to 0.0 (no effect).
#[derive(Debug, Clone)]
pub struct ChaosConfig {
    /// Base latency added to all packets (default: 0ms)
    pub latency: Duration,

    /// Maximum random jitter added/subtracted from latency (default: 0ms)
    /// Actual jitter is uniformly distributed in [-jitter, +jitter]
    pub jitter: Duration,

    /// Probability of dropping a packet on send (0.0 - 1.0, default: 0.0)
    pub send_loss_rate: f64,

    /// Probability of dropping a packet on receive (0.0 - 1.0, default: 0.0)
    pub receive_loss_rate: f64,

    /// Probability of duplicating a packet (0.0 - 1.0, default: 0.0)
    pub duplication_rate: f64,

    /// Number of packets to buffer before potentially reordering (default: 0)
    /// When > 0, packets are buffered and may be delivered out of order
    pub reorder_buffer_size: usize,

    /// Probability of reordering packets within the buffer (0.0 - 1.0, default: 0.0)
    pub reorder_rate: f64,

    /// Probability of starting a burst loss event (0.0 - 1.0, default: 0.0)
    /// When triggered, drops `burst_loss_length` consecutive packets
    pub burst_loss_probability: f64,

    /// Number of consecutive packets to drop during a burst loss event (default: 0)
    pub burst_loss_length: usize,

    /// Random seed for deterministic behavior (default: random)
    pub seed: Option<u64>,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            latency: Duration::ZERO,
            jitter: Duration::ZERO,
            send_loss_rate: 0.0,
            receive_loss_rate: 0.0,
            duplication_rate: 0.0,
            reorder_buffer_size: 0,
            reorder_rate: 0.0,
            burst_loss_probability: 0.0,
            burst_loss_length: 0,
            seed: None,
        }
    }
}

impl ChaosConfig {
    /// Creates a new builder for fluent configuration.
    pub fn builder() -> ChaosConfigBuilder {
        ChaosConfigBuilder::new()
    }

    /// Creates a config with no chaos (passthrough mode).
    pub fn passthrough() -> Self {
        Self::default()
    }

    /// Creates a config simulating high latency conditions.
    pub fn high_latency(latency_ms: u64) -> Self {
        Self {
            latency: Duration::from_millis(latency_ms),
            ..Default::default()
        }
    }

    /// Creates a config simulating packet loss.
    pub fn lossy(loss_rate: f64) -> Self {
        Self {
            send_loss_rate: loss_rate,
            receive_loss_rate: loss_rate,
            ..Default::default()
        }
    }

    /// Creates a config simulating typical poor network conditions.
    pub fn poor_network() -> Self {
        Self {
            latency: Duration::from_millis(100),
            jitter: Duration::from_millis(50),
            send_loss_rate: 0.05,
            receive_loss_rate: 0.05,
            ..Default::default()
        }
    }

    /// Creates a config simulating very bad network conditions.
    pub fn terrible_network() -> Self {
        Self {
            latency: Duration::from_millis(250),
            jitter: Duration::from_millis(100),
            send_loss_rate: 0.15,
            receive_loss_rate: 0.15,
            duplication_rate: 0.02,
            reorder_buffer_size: 5,
            reorder_rate: 0.1,
            ..Default::default()
        }
    }
}

/// Builder for [`ChaosConfig`].
#[derive(Debug, Clone, Default)]
pub struct ChaosConfigBuilder {
    config: ChaosConfig,
}

impl ChaosConfigBuilder {
    /// Creates a new builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the base latency for all packets.
    pub fn latency(mut self, latency: Duration) -> Self {
        self.config.latency = latency;
        self
    }

    /// Sets the latency in milliseconds (convenience method).
    pub fn latency_ms(mut self, ms: u64) -> Self {
        self.config.latency = Duration::from_millis(ms);
        self
    }

    /// Sets the maximum jitter (random variation in latency).
    pub fn jitter(mut self, jitter: Duration) -> Self {
        self.config.jitter = jitter;
        self
    }

    /// Sets the jitter in milliseconds (convenience method).
    pub fn jitter_ms(mut self, ms: u64) -> Self {
        self.config.jitter = Duration::from_millis(ms);
        self
    }

    /// Sets packet loss rate for both send and receive.
    pub fn packet_loss_rate(mut self, rate: f64) -> Self {
        self.config.send_loss_rate = rate.clamp(0.0, 1.0);
        self.config.receive_loss_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Sets packet loss rate for sending only.
    pub fn send_loss_rate(mut self, rate: f64) -> Self {
        self.config.send_loss_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Sets packet loss rate for receiving only.
    pub fn receive_loss_rate(mut self, rate: f64) -> Self {
        self.config.receive_loss_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Sets the packet duplication rate.
    pub fn duplication_rate(mut self, rate: f64) -> Self {
        self.config.duplication_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Sets the reorder buffer size.
    pub fn reorder_buffer_size(mut self, size: usize) -> Self {
        self.config.reorder_buffer_size = size;
        self
    }

    /// Sets the reorder rate (probability of swapping within buffer).
    pub fn reorder_rate(mut self, rate: f64) -> Self {
        self.config.reorder_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Sets burst loss parameters.
    ///
    /// When a burst is triggered (with `probability`), `length` consecutive
    /// packets will be dropped.
    pub fn burst_loss(mut self, probability: f64, length: usize) -> Self {
        self.config.burst_loss_probability = probability.clamp(0.0, 1.0);
        self.config.burst_loss_length = length;
        self
    }

    /// Sets the random seed for deterministic behavior.
    pub fn seed(mut self, seed: u64) -> Self {
        self.config.seed = Some(seed);
        self
    }

    /// Builds the configuration.
    pub fn build(self) -> ChaosConfig {
        self.config
    }
}

/// A packet in flight with its scheduled delivery time.
#[derive(Debug, Clone)]
struct InFlightPacket<A> {
    addr: A,
    msg: Message,
    deliver_at: Instant,
}

/// A socket wrapper that injects configurable network chaos.
///
/// Wraps any [`NonBlockingSocket`] implementation to simulate adverse
/// network conditions for testing purposes.
///
/// # Type Parameters
///
/// - `A`: The address type (must match the inner socket)
/// - `S`: The inner socket type implementing [`NonBlockingSocket<A>`]
///
/// # Thread Safety
///
/// When the `sync-send` feature is enabled, `ChaosSocket` implements
/// `Send + Sync` if the inner socket does.
pub struct ChaosSocket<A, S>
where
    A: Clone + PartialEq + Eq + Hash + Send + Sync,
    S: NonBlockingSocket<A>,
{
    inner: S,
    config: ChaosConfig,
    rng: SmallRng,

    /// Packets waiting to be delivered (simulating latency)
    in_flight: VecDeque<InFlightPacket<A>>,

    /// Buffer for potential reordering on receive
    reorder_buffer: Vec<(A, Message)>,

    /// Remaining packets to drop in current burst loss event
    burst_loss_remaining: usize,

    /// Statistics tracking
    stats: ChaosStats,
}

/// Statistics about chaos socket behavior.
#[derive(Debug, Clone, Default)]
pub struct ChaosStats {
    /// Total packets sent through the socket
    pub packets_sent: u64,
    /// Packets dropped on send
    pub packets_dropped_send: u64,
    /// Packets duplicated on send
    pub packets_duplicated: u64,
    /// Total packets received
    pub packets_received: u64,
    /// Packets dropped on receive
    pub packets_dropped_receive: u64,
    /// Packets reordered
    pub packets_reordered: u64,
    /// Number of burst loss events triggered
    pub burst_loss_events: u64,
    /// Packets dropped due to burst loss
    pub packets_dropped_burst: u64,
}

impl<A, S> ChaosSocket<A, S>
where
    A: Clone + PartialEq + Eq + Hash + Send + Sync,
    S: NonBlockingSocket<A>,
{
    /// Creates a new chaos socket wrapping the given inner socket.
    pub fn new(inner: S, config: ChaosConfig) -> Self {
        let rng = match config.seed {
            Some(seed) => SmallRng::seed_from_u64(seed),
            None => SmallRng::from_entropy(),
        };

        Self {
            inner,
            config,
            rng,
            in_flight: VecDeque::new(),
            reorder_buffer: Vec::new(),
            burst_loss_remaining: 0,
            stats: ChaosStats::default(),
        }
    }

    /// Returns a reference to the inner socket.
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Returns a mutable reference to the inner socket.
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Consumes the chaos socket and returns the inner socket.
    pub fn into_inner(self) -> S {
        self.inner
    }

    /// Returns the current chaos configuration.
    pub fn config(&self) -> &ChaosConfig {
        &self.config
    }

    /// Updates the chaos configuration.
    pub fn set_config(&mut self, config: ChaosConfig) {
        self.config = config;
    }

    /// Returns statistics about chaos behavior.
    pub fn stats(&self) -> &ChaosStats {
        &self.stats
    }

    /// Resets statistics counters.
    pub fn reset_stats(&mut self) {
        self.stats = ChaosStats::default();
    }

    /// Returns the number of packets currently in flight (delayed).
    pub fn packets_in_flight(&self) -> usize {
        self.in_flight.len()
    }

    /// Calculates the delivery time for a packet with latency and jitter.
    fn calculate_delivery_time(&mut self) -> Instant {
        let base_latency = self.config.latency;
        let jitter = if self.config.jitter > Duration::ZERO {
            let jitter_range = self.config.jitter.as_nanos() as i64;
            let jitter_offset = self.rng.gen_range(-jitter_range..=jitter_range);
            if jitter_offset >= 0 {
                Duration::from_nanos(jitter_offset as u64)
            } else {
                // Negative jitter reduces latency but not below zero
                let reduction = Duration::from_nanos((-jitter_offset) as u64);
                if reduction > base_latency {
                    return Instant::now(); // Clamp to now
                }
                return Instant::now() + base_latency - reduction;
            }
        } else {
            Duration::ZERO
        };

        Instant::now() + base_latency + jitter
    }

    /// Determines if a packet should be dropped based on the given rate.
    fn should_drop(&mut self, rate: f64) -> bool {
        if rate <= 0.0 {
            false
        } else if rate >= 1.0 {
            true
        } else {
            self.rng.gen::<f64>() < rate
        }
    }

    /// Determines if a packet should be duplicated.
    fn should_duplicate(&mut self) -> bool {
        self.should_drop(self.config.duplication_rate)
    }

    /// Determines if a packet should be dropped due to burst loss.
    /// Returns true if the packet should be dropped.
    fn should_drop_burst(&mut self) -> bool {
        // If we're in a burst, continue dropping
        if self.burst_loss_remaining > 0 {
            self.burst_loss_remaining -= 1;
            self.stats.packets_dropped_burst += 1;
            return true;
        }

        // Check if we should start a new burst
        if self.config.burst_loss_length > 0 && self.should_drop(self.config.burst_loss_probability)
        {
            self.stats.burst_loss_events += 1;
            // Drop this packet and set up remaining burst
            self.burst_loss_remaining = self.config.burst_loss_length.saturating_sub(1);
            self.stats.packets_dropped_burst += 1;
            return true;
        }

        false
    }

    /// Delivers packets that have reached their delivery time.
    fn deliver_ready_packets(&mut self) -> Vec<(A, Message)> {
        let now = Instant::now();
        let mut ready = Vec::new();

        while let Some(packet) = self.in_flight.front() {
            if packet.deliver_at <= now {
                let packet = self.in_flight.pop_front().unwrap();
                ready.push((packet.addr, packet.msg));
            } else {
                break;
            }
        }

        ready
    }

    /// Applies reordering to a batch of messages.
    fn apply_reordering(&mut self, messages: &mut Vec<(A, Message)>) {
        if self.config.reorder_buffer_size == 0 || self.config.reorder_rate <= 0.0 {
            return;
        }

        // Add messages to reorder buffer
        self.reorder_buffer.append(messages);

        // If buffer is full enough, potentially reorder and release
        if self.reorder_buffer.len() >= self.config.reorder_buffer_size {
            // Apply random swaps based on reorder_rate
            for i in 0..self.reorder_buffer.len() {
                if self.should_drop(self.config.reorder_rate) {
                    let j = self.rng.gen_range(0..self.reorder_buffer.len());
                    if i != j {
                        self.reorder_buffer.swap(i, j);
                        self.stats.packets_reordered += 1;
                    }
                }
            }

            // Release all buffered packets
            messages.append(&mut self.reorder_buffer);
        }
    }
}

// Implementation for sync-send feature
#[cfg(feature = "sync-send")]
impl<A, S> NonBlockingSocket<A> for ChaosSocket<A, S>
where
    A: Clone + PartialEq + Eq + Hash + Send + Sync,
    S: NonBlockingSocket<A> + Send + Sync,
{
    fn send_to(&mut self, msg: &Message, addr: &A) {
        self.stats.packets_sent += 1;

        // Check for burst loss first (takes priority)
        if self.should_drop_burst() {
            return;
        }

        // Check for packet loss on send
        if self.should_drop(self.config.send_loss_rate) {
            self.stats.packets_dropped_send += 1;
            return;
        }

        // Send immediately to inner socket
        self.inner.send_to(msg, addr);

        // Check for duplication - send additional copy
        if self.should_duplicate() {
            self.stats.packets_duplicated += 1;
            self.inner.send_to(msg, addr);
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(A, Message)> {
        // Receive new messages from the inner socket
        let new_messages = self.inner.receive_all_messages();

        // Queue new messages with latency
        for (addr, msg) in new_messages {
            // Apply receive-side packet loss before queueing
            if self.should_drop(self.config.receive_loss_rate) {
                self.stats.packets_dropped_receive += 1;
                continue;
            }

            let deliver_at = self.calculate_delivery_time();
            self.in_flight.push_back(InFlightPacket {
                addr,
                msg,
                deliver_at,
            });
        }

        // Sort by delivery time to maintain order (unless reordering is enabled)
        if self.config.reorder_rate <= 0.0 {
            self.in_flight
                .make_contiguous()
                .sort_by_key(|p| p.deliver_at);
        }

        // Deliver packets that have completed their latency delay
        let mut ready = self.deliver_ready_packets();
        self.stats.packets_received += ready.len() as u64;

        // Apply reordering to ready packets
        self.apply_reordering(&mut ready);

        ready
    }
}

// Implementation for non sync-send feature
#[cfg(not(feature = "sync-send"))]
impl<A, S> NonBlockingSocket<A> for ChaosSocket<A, S>
where
    A: Clone + PartialEq + Eq + Hash + Send + Sync,
    S: NonBlockingSocket<A>,
{
    fn send_to(&mut self, msg: &Message, addr: &A) {
        self.stats.packets_sent += 1;

        // Check for burst loss first (takes priority)
        if self.should_drop_burst() {
            return;
        }

        // Check for packet loss on send
        if self.should_drop(self.config.send_loss_rate) {
            self.stats.packets_dropped_send += 1;
            return;
        }

        // Send immediately to inner socket
        self.inner.send_to(msg, addr);

        // Check for duplication - send additional copy
        if self.should_duplicate() {
            self.stats.packets_duplicated += 1;
            self.inner.send_to(msg, addr);
        }
    }

    fn receive_all_messages(&mut self) -> Vec<(A, Message)> {
        // Receive new messages from the inner socket
        let new_messages = self.inner.receive_all_messages();

        // Queue new messages with latency
        for (addr, msg) in new_messages {
            // Apply receive-side packet loss before queueing
            if self.should_drop(self.config.receive_loss_rate) {
                self.stats.packets_dropped_receive += 1;
                continue;
            }

            let deliver_at = self.calculate_delivery_time();
            self.in_flight.push_back(InFlightPacket {
                addr,
                msg,
                deliver_at,
            });
        }

        // Sort by delivery time to maintain order (unless reordering is enabled)
        if self.config.reorder_rate <= 0.0 {
            self.in_flight
                .make_contiguous()
                .sort_by_key(|p| p.deliver_at);
        }

        // Deliver packets that have completed their latency delay
        let mut ready = self.deliver_ready_packets();
        self.stats.packets_received += ready.len() as u64;

        // Apply reordering to ready packets
        self.apply_reordering(&mut ready);

        ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    /// A simple in-memory socket for testing.
    #[derive(Default)]
    struct TestSocket {
        sent: Vec<(SocketAddr, Message)>,
        to_receive: Vec<(SocketAddr, Message)>,
    }

    impl NonBlockingSocket<SocketAddr> for TestSocket {
        fn send_to(&mut self, msg: &Message, addr: &SocketAddr) {
            self.sent.push((*addr, msg.clone()));
        }

        fn receive_all_messages(&mut self) -> Vec<(SocketAddr, Message)> {
            std::mem::take(&mut self.to_receive)
        }
    }

    fn test_addr() -> SocketAddr {
        "127.0.0.1:8080".parse().unwrap()
    }

    fn test_message() -> Message {
        use crate::network::messages::{MessageBody, MessageHeader};

        Message {
            header: MessageHeader { magic: 0 },
            body: MessageBody::KeepAlive,
        }
    }

    #[test]
    fn test_passthrough_mode() {
        let inner = TestSocket::default();
        let mut socket = ChaosSocket::new(inner, ChaosConfig::passthrough());

        let addr = test_addr();
        let msg = test_message();

        socket.send_to(&msg, &addr);

        assert_eq!(socket.stats().packets_sent, 1);
        assert_eq!(socket.stats().packets_dropped_send, 0);
        assert_eq!(socket.inner().sent.len(), 1);
    }

    #[test]
    fn test_packet_loss_100_percent() {
        let inner = TestSocket::default();
        let config = ChaosConfig::builder().packet_loss_rate(1.0).seed(42).build();
        let mut socket = ChaosSocket::new(inner, config);

        let addr = test_addr();
        let msg = test_message();

        // Send 10 packets
        for _ in 0..10 {
            socket.send_to(&msg, &addr);
        }

        // All should be dropped
        assert_eq!(socket.stats().packets_sent, 10);
        assert_eq!(socket.stats().packets_dropped_send, 10);
        assert_eq!(socket.inner().sent.len(), 0);
    }

    #[test]
    fn test_packet_loss_partial() {
        let inner = TestSocket::default();
        let config = ChaosConfig::builder().packet_loss_rate(0.5).seed(42).build();
        let mut socket = ChaosSocket::new(inner, config);

        let addr = test_addr();
        let msg = test_message();

        // Send 100 packets
        for _ in 0..100 {
            socket.send_to(&msg, &addr);
        }

        // Approximately half should be dropped (with seed 42)
        let dropped = socket.stats().packets_dropped_send;
        let sent_through = socket.inner().sent.len();

        assert_eq!(dropped + sent_through as u64, 100);
        // With 50% loss, expect roughly 40-60 dropped
        assert!(dropped > 30, "Expected more drops, got {}", dropped);
        assert!(dropped < 70, "Expected fewer drops, got {}", dropped);
    }

    #[test]
    fn test_receive_loss() {
        let mut inner = TestSocket::default();
        let addr = test_addr();
        let msg = test_message();

        // Queue 10 messages to receive
        for _ in 0..10 {
            inner.to_receive.push((addr, msg.clone()));
        }

        let config = ChaosConfig::builder()
            .receive_loss_rate(1.0)
            .seed(42)
            .build();
        let mut socket = ChaosSocket::new(inner, config);

        let received = socket.receive_all_messages();

        assert_eq!(received.len(), 0);
        assert_eq!(socket.stats().packets_dropped_receive, 10);
    }

    #[test]
    fn test_duplication() {
        let inner = TestSocket::default();
        let config = ChaosConfig::builder()
            .duplication_rate(1.0)
            .seed(42)
            .build();
        let mut socket = ChaosSocket::new(inner, config);

        let addr = test_addr();
        let msg = test_message();

        socket.send_to(&msg, &addr);

        // Should have sent twice (original + duplicate)
        assert_eq!(socket.inner().sent.len(), 2);
        assert_eq!(socket.stats().packets_duplicated, 1);
    }

    #[test]
    fn test_config_builder() {
        let config = ChaosConfig::builder()
            .latency_ms(100)
            .jitter_ms(20)
            .packet_loss_rate(0.1)
            .duplication_rate(0.05)
            .reorder_buffer_size(5)
            .reorder_rate(0.2)
            .seed(12345)
            .build();

        assert_eq!(config.latency, Duration::from_millis(100));
        assert_eq!(config.jitter, Duration::from_millis(20));
        assert_eq!(config.send_loss_rate, 0.1);
        assert_eq!(config.receive_loss_rate, 0.1);
        assert_eq!(config.duplication_rate, 0.05);
        assert_eq!(config.reorder_buffer_size, 5);
        assert_eq!(config.reorder_rate, 0.2);
        assert_eq!(config.seed, Some(12345));
    }

    #[test]
    fn test_preset_configs() {
        let poor = ChaosConfig::poor_network();
        assert_eq!(poor.latency, Duration::from_millis(100));
        assert_eq!(poor.send_loss_rate, 0.05);

        let terrible = ChaosConfig::terrible_network();
        assert_eq!(terrible.latency, Duration::from_millis(250));
        assert_eq!(terrible.send_loss_rate, 0.15);
        assert!(terrible.reorder_buffer_size > 0);
    }

    #[test]
    fn test_stats_tracking() {
        let inner = TestSocket::default();
        let config = ChaosConfig::builder()
            .packet_loss_rate(0.5)
            .seed(42)
            .build();
        let mut socket = ChaosSocket::new(inner, config);

        let addr = test_addr();
        let msg = test_message();

        for _ in 0..10 {
            socket.send_to(&msg, &addr);
        }

        let stats = socket.stats();
        assert_eq!(stats.packets_sent, 10);
        assert!(stats.packets_dropped_send > 0);

        socket.reset_stats();
        assert_eq!(socket.stats().packets_sent, 0);
    }

    #[test]
    fn test_deterministic_with_seed() {
        let run_test = |seed: u64| -> u64 {
            let inner = TestSocket::default();
            let config = ChaosConfig::builder().packet_loss_rate(0.5).seed(seed).build();
            let mut socket = ChaosSocket::new(inner, config);

            let addr = test_addr();
            let msg = test_message();

            for _ in 0..100 {
                socket.send_to(&msg, &addr);
            }

            socket.stats().packets_dropped_send
        };

        // Same seed should produce same results
        let result1 = run_test(42);
        let result2 = run_test(42);
        assert_eq!(result1, result2);

        // Different seed should (very likely) produce different results
        let result3 = run_test(123);
        assert_ne!(result1, result3);
    }

    #[test]
    fn test_asymmetric_loss() {
        let config = ChaosConfig::builder()
            .send_loss_rate(0.8)
            .receive_loss_rate(0.2)
            .build();

        assert_eq!(config.send_loss_rate, 0.8);
        assert_eq!(config.receive_loss_rate, 0.2);
    }

    #[test]
    fn test_loss_rate_clamping() {
        let config = ChaosConfig::builder()
            .packet_loss_rate(1.5) // Should clamp to 1.0
            .build();

        assert_eq!(config.send_loss_rate, 1.0);
        assert_eq!(config.receive_loss_rate, 1.0);

        let config2 = ChaosConfig::builder()
            .packet_loss_rate(-0.5) // Should clamp to 0.0
            .build();

        assert_eq!(config2.send_loss_rate, 0.0);
        assert_eq!(config2.receive_loss_rate, 0.0);
    }

    #[test]
    fn test_latency_delays_delivery() {
        let mut inner = TestSocket::default();
        let addr = test_addr();
        let msg = test_message();

        // Queue a message to receive
        inner.to_receive.push((addr, msg.clone()));

        // Set up high latency (100ms)
        let config = ChaosConfig::builder()
            .latency_ms(100)
            .seed(42)
            .build();
        let mut socket = ChaosSocket::new(inner, config);

        // First receive - packet goes into in-flight queue
        let received = socket.receive_all_messages();
        assert_eq!(received.len(), 0, "Packet should be delayed");
        assert_eq!(socket.packets_in_flight(), 1, "Packet should be in flight");

        // Wait less than latency - still not delivered
        std::thread::sleep(Duration::from_millis(50));
        let received = socket.receive_all_messages();
        assert_eq!(received.len(), 0, "Packet should still be delayed");

        // Wait for full latency - now delivered
        std::thread::sleep(Duration::from_millis(60));
        let received = socket.receive_all_messages();
        assert_eq!(received.len(), 1, "Packet should be delivered after latency");
        assert_eq!(socket.packets_in_flight(), 0, "No more packets in flight");
    }

    #[test]
    fn test_zero_latency_immediate_delivery() {
        let mut inner = TestSocket::default();
        let addr = test_addr();
        let msg = test_message();

        // Queue messages to receive
        for _ in 0..5 {
            inner.to_receive.push((addr, msg.clone()));
        }

        // Passthrough mode (zero latency)
        let mut socket = ChaosSocket::new(inner, ChaosConfig::passthrough());

        // Messages should be delivered immediately (within tiny jitter of Instant::now())
        let received = socket.receive_all_messages();
        assert_eq!(received.len(), 5, "All packets should be delivered immediately");
    }

    #[test]
    fn test_in_flight_count() {
        let mut inner = TestSocket::default();
        let addr = test_addr();
        let msg = test_message();

        // Queue 5 messages
        for _ in 0..5 {
            inner.to_receive.push((addr, msg.clone()));
        }

        let config = ChaosConfig::builder().latency_ms(100).build();
        let mut socket = ChaosSocket::new(inner, config);

        // Receive puts them in flight
        let _ = socket.receive_all_messages();
        assert_eq!(socket.packets_in_flight(), 5);

        // Wait and check they're delivered
        std::thread::sleep(Duration::from_millis(110));
        let received = socket.receive_all_messages();
        assert_eq!(received.len(), 5);
        assert_eq!(socket.packets_in_flight(), 0);
    }

    #[test]
    fn test_burst_loss_drops_consecutive_packets() {
        let inner = TestSocket::default();
        // 100% probability of burst, 5 packets per burst
        let config = ChaosConfig::builder()
            .burst_loss(1.0, 5)
            .seed(42)
            .build();
        let mut socket = ChaosSocket::new(inner, config);

        let addr = test_addr();
        let msg = test_message();

        // Send 10 packets - first 5 should be dropped (burst), next 5 should trigger another burst
        for _ in 0..10 {
            socket.send_to(&msg, &addr);
        }

        // All 10 packets should be dropped due to burst loss
        assert_eq!(socket.stats().packets_sent, 10);
        assert_eq!(socket.stats().packets_dropped_burst, 10);
        assert_eq!(socket.stats().burst_loss_events, 2); // Two bursts of 5
        assert_eq!(socket.inner().sent.len(), 0);
    }

    #[test]
    fn test_burst_loss_partial_probability() {
        let inner = TestSocket::default();
        // 50% probability of starting a burst, 3 packets per burst
        let config = ChaosConfig::builder()
            .burst_loss(0.5, 3)
            .seed(42)
            .build();
        let mut socket = ChaosSocket::new(inner, config);

        let addr = test_addr();
        let msg = test_message();

        // Send 20 packets
        for _ in 0..20 {
            socket.send_to(&msg, &addr);
        }

        // Some packets should be dropped in bursts
        let dropped = socket.stats().packets_dropped_burst;
        let sent_through = socket.inner().sent.len();
        let burst_events = socket.stats().burst_loss_events;

        // Should have some burst events
        assert!(burst_events > 0, "Expected at least one burst event");
        // Each burst drops 3 packets
        assert_eq!(
            dropped,
            burst_events * 3,
            "Burst dropped {} but expected {} (events * 3)",
            dropped,
            burst_events * 3
        );
        // Total should add up
        assert_eq!(
            dropped + sent_through as u64,
            20,
            "dropped + sent should equal 20"
        );
    }

    #[test]
    fn test_burst_loss_builder() {
        let config = ChaosConfig::builder()
            .burst_loss(0.2, 8)
            .build();

        assert_eq!(config.burst_loss_probability, 0.2);
        assert_eq!(config.burst_loss_length, 8);
    }

    #[test]
    fn test_burst_loss_stats_tracking() {
        let inner = TestSocket::default();
        let config = ChaosConfig::builder()
            .burst_loss(1.0, 3)
            .seed(42)
            .build();
        let mut socket = ChaosSocket::new(inner, config);

        let addr = test_addr();
        let msg = test_message();

        // Send exactly 3 packets (one burst)
        for _ in 0..3 {
            socket.send_to(&msg, &addr);
        }

        assert_eq!(socket.stats().burst_loss_events, 1);
        assert_eq!(socket.stats().packets_dropped_burst, 3);

        // Reset and verify stats cleared
        socket.reset_stats();
        assert_eq!(socket.stats().burst_loss_events, 0);
        assert_eq!(socket.stats().packets_dropped_burst, 0);
    }
}
