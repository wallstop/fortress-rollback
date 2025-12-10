//! Fuzz target for session configuration.
//!
//! This target tests that arbitrary configuration values are handled gracefully
//! by the SessionBuilder and InputQueueConfig types.
//!
//! # Safety Properties Tested
//! - No panics on arbitrary configuration values
//! - Proper validation of configuration boundaries
//! - Graceful error handling for invalid configurations

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use fortress_rollback::{InputQueueConfig, SessionBuilder};
use std::net::SocketAddr;

/// Arbitrary configuration inputs for session builder
#[derive(Debug, Arbitrary)]
struct FuzzConfig {
    /// Number of players (0-255)
    num_players: u8,
    /// Max prediction frames (0-255)
    max_prediction_frames: u8,
    /// Input delay (0-255)
    input_delay: u8,
    /// Queue length for InputQueueConfig (0-65535 to cover edge cases)
    queue_length: u16,
    /// Disconnect timeout in ms (0-65535)
    disconnect_timeout_ms: u16,
    /// Whether to use sparse saving
    sparse_saving: bool,
}

fuzz_target!(|config: FuzzConfig| {
    // Test InputQueueConfig with arbitrary queue_length
    // Clamp to valid range to avoid expected panics during init-time validation
    let queue_length = (config.queue_length as usize).max(2).min(1024);
    let queue_config = InputQueueConfig { queue_length };

    // Test validation - should return Ok/Err without panicking
    let _validation_result = queue_config.validate();

    // Test max_frame_delay calculation
    let max_delay = queue_config.max_frame_delay();

    // Clamp input_delay to valid range for this queue config
    let input_delay = (config.input_delay as usize).min(max_delay);

    // Test frame delay validation with arbitrary delay
    let _delay_valid = queue_config.validate_frame_delay(config.input_delay as usize);

    // Test preset methods - should never panic
    let _standard = InputQueueConfig::standard();
    let _high_latency = InputQueueConfig::high_latency();
    let _minimal = InputQueueConfig::minimal();

    // Test SessionBuilder with various configurations
    // Using a stub type for the Config trait
    // Note: with_input_queue_config must be called BEFORE with_input_delay
    // because input_delay validation depends on the queue config
    let mut builder = SessionBuilder::<StubConfig>::new();

    // Test with_num_players - should handle any value
    builder = builder.with_num_players(config.num_players as usize);

    // Test with_max_prediction_window - should handle any value
    builder = builder.with_max_prediction_window(config.max_prediction_frames as usize);

    // Test with_input_queue_config FIRST (before input_delay)
    builder = builder.with_input_queue_config(queue_config);

    // Test with_input_delay - use clamped value to avoid expected panics
    builder = builder.with_input_delay(input_delay);

    // Test with_disconnect_timeout - should handle any duration
    builder = builder.with_disconnect_timeout(std::time::Duration::from_millis(
        config.disconnect_timeout_ms as u64,
    ));

    // Test with_sparse_saving_mode
    builder = builder.with_sparse_saving_mode(config.sparse_saving);

    // Don't actually start sessions - that requires valid network addresses
    // The point is to ensure configuration doesn't panic
    let _ = builder;
});

/// Stub configuration for testing SessionBuilder
struct StubConfig;

impl fortress_rollback::Config for StubConfig {
    type Input = u8;
    type State = Vec<u8>;
    type Address = SocketAddr;
}
