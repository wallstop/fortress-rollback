//! Extraction wrapper for the #365 varint Lean pilot.
//!
//! The module below is the unchanged production `src/rle/varint.rs`; the
//! wrappers only make its `pub(crate)` surface reachable for Hax/Aeneas.

#![allow(private_interfaces)]

#[path = "../../../../../src/rle/varint.rs"]
mod varint;

pub fn encoded_len_of(value: u64) -> usize {
    varint::encoded_len(value)
}

pub fn encode_into(value: u64, buf: &mut [u8]) -> usize {
    varint::encode(value, buf)
}

pub fn decode_from(buf: &[u8], offset: usize) -> Result<(u64, usize), varint::VarintError> {
    varint::decode(buf, offset)
}
