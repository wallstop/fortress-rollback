//! Fuzz target for network message deserialization.
//!
//! This target tests that arbitrary bytes cannot crash the message deserializer.
//! It ensures that malformed or malicious network data is handled gracefully
//! without panicking or causing undefined behavior.
//!
//! # Safety Properties Tested
//! - No panics on arbitrary input
//! - No unbounded memory allocation
//! - Graceful error handling for invalid data

#![no_main]

use libfuzzer_sys::fuzz_target;

// Import the Message type and codec from the public API
use fortress_rollback::network::codec;
use fortress_rollback::Message;

fuzz_target!(|data: &[u8]| {
    // Test codec deserialization - should never panic
    // Even malformed data should return Err, not panic
    let _result: Result<Message, _> = codec::decode_value(data);

    // If deserialization succeeded, ensure re-serialization works
    if let Ok(msg) = codec::decode_value::<Message>(data) {
        // Round-trip should work without panicking
        let _serialized = codec::encode(&msg);
    }
});
