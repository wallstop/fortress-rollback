//! Fuzz target for replay deserialization.
//!
//! This target tests that arbitrary bytes cannot crash, abort, or
//! over-allocate in the replay decoder. A malformed or malicious encoded
//! replay must be handled gracefully -- returning an error rather than
//! panicking, aborting via an allocator failure, or driving an
//! out-of-proportion speculative allocation from an attacker-chosen length
//! prefix (the RUSTSEC-2022-0035 class). Any replay that *does* decode must
//! re-serialize without panicking.
//!
//! # Safety Properties Tested
//! - No panics on arbitrary input
//! - No unbounded / over-proportion memory allocation
//! - Graceful error handling for invalid data
//! - Decoded replays round-trip back through `to_bytes`

#![no_main]

use libfuzzer_sys::fuzz_target;

// `u32` is a multi-byte input type, so this also exercises the inner per-frame
// input parsing path rather than only the length-prefix bounds.
use fortress_rollback::replay::Replay;

fuzz_target!(|data: &[u8]| {
    // Decoding arbitrary bytes must never panic/abort/over-allocate; it must
    // return Err on malformed input instead.
    if let Ok(replay) = Replay::<u32>::from_bytes(data) {
        // A successfully decoded replay must re-serialize without panicking.
        let _ = replay.to_bytes();
    }
});
