//! Minimal `wasm32-wasip1` runtime probe for the portable replay fixture.
//!
//! Compile-only wasm evidence cannot catch a `usize`-width, endianness, or
//! WASM-boundary regression in the code paths the fixture claims to cover.
//! This binary runs the same replay, machine-width, compression, and wire
//! codec checks as the browser and Emscripten lanes under wasmtime.

// The probe prints its evidence directly; it is not a library hot path.
#![allow(clippy::disallowed_macros)]

use fortress_replay_fixture::{
    EXPECTED_DIGEST, EXPECTED_FRAMES, EXPECTED_REQUEST_DIGEST, EXPECTED_WIRE_CODEC_DIGEST,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("wasip1 replay probe failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    fortress_replay_fixture::check_machine_width_parity()?;
    fortress_replay_fixture::check_compression_parity()?;
    if !fortress_replay_fixture::check_wire_codec_digest() {
        return Err(format!(
            "coordinated-drop wire codec digest differs from native {EXPECTED_WIRE_CODEC_DIGEST:#018x}"
        ));
    }
    let evidence = fortress_replay_fixture::replay_fixture()?;
    if evidence.frames != EXPECTED_FRAMES {
        return Err(format!(
            "captured replay played {} frames, expected {EXPECTED_FRAMES}",
            evidence.frames
        ));
    }
    if evidence.digest != EXPECTED_DIGEST {
        return Err(format!(
            "captured replay digest {:#034x} differs from native {EXPECTED_DIGEST:#034x}",
            evidence.digest
        ));
    }
    if evidence.request_digest != EXPECTED_REQUEST_DIGEST {
        return Err(format!(
            "captured replay request digest {:#034x} differs from native \
             {EXPECTED_REQUEST_DIGEST:#034x}",
            evidence.request_digest
        ));
    }
    if evidence.desyncs != 0 {
        return Err(format!(
            "captured replay reported {} checksum desync(s) under wasip1",
            evidence.desyncs
        ));
    }
    println!(
        "wasip1 replay ok: frames={} requests={} digest={:#034x} request_digest={:#034x}",
        evidence.frames, evidence.requests, evidence.digest, evidence.request_digest
    );
    Ok(())
}
