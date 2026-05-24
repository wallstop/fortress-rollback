//! Fuzz target for protocol input packet acceptance.
//!
//! This target feeds arbitrary frame numbers, ack frames, status vectors, and
//! compressed payload bytes into the same internal `UdpProtocol::on_input` path
//! used for received input packets. The safety contract is no panic, bounded
//! decode/history growth, and clean rejection of malformed packets.

#![no_main]

use arbitrary::Arbitrary;
use fortress_rollback::__internal::fuzz_protocol_input_packet;
use libfuzzer_sys::fuzz_target;

#[derive(Debug, Arbitrary)]
struct StatusInput {
    disconnected: bool,
    last_frame: i32,
}

#[derive(Debug, Arbitrary)]
struct ProtocolInputPacket {
    start_frame: i32,
    ack_frame: i32,
    peer_connect_status: Vec<StatusInput>,
    bytes: Vec<u8>,
    pending_frames: Vec<i32>,
}

fuzz_target!(|packet: ProtocolInputPacket| {
    let status_len = packet.peer_connect_status.len().min(8);
    let mut statuses = Vec::new();
    if statuses.try_reserve_exact(status_len).is_err() {
        return;
    }
    for status in packet.peer_connect_status.iter().take(status_len) {
        statuses.push((status.disconnected, status.last_frame));
    }

    fuzz_protocol_input_packet(
        packet.start_frame,
        packet.ack_frame,
        &statuses,
        &packet.bytes,
        &packet.pending_frames,
    );
});
