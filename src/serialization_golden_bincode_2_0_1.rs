//! Immutable bincode 2.0.1 serialization fixtures.
//!
//! These literals freeze the non-`Message` compatibility surfaces that share
//! Fortress's codec: fixed-width game input, rich saved state, hot-join state
//! and bridge payloads, replay files, and state-derived checksums. Protocol
//! `Message` bytes remain exhaustively covered by `network::wire_golden_v2`.
//!
//! Do not rewrite this file. A future serializer candidate must preserve these
//! bytes or add a separately versioned format and migration decoder while this
//! historical suite continues to pass.

use crate::checksum::{
    compute_checksum, compute_checksum_fletcher16, fletcher16, hash_bytes_fnv1a,
};
use crate::network::codec;
use crate::replay::{Replay, ReplayMetadata};
#[cfg(feature = "hot-join")]
use crate::{frame_info::PlayerInput, Config, Frame};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;

const BINCODE_GOLDEN_VERSION: &str = "2.0.1";

const INPUT_A_BYTES: &[u8] = &[
    0x34, 0x12, 0x2E, 0xFB, 0xAA, 0x55, 0x7F, 0x01, 0xEF, 0xCD, 0xAB, 0x89, 0x01, 0x00, 0x00, 0x00,
];
const INPUT_B_BYTES: &[u8] = &[
    0xCD, 0xAB, 0xE1, 0x10, 0x01, 0x02, 0x03, 0x00, 0x40, 0x30, 0x20, 0x10, 0x00, 0x00, 0x00, 0x00,
];
const STATE_BYTES: &[u8] = &[
    0x04, 0x03, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
    0x90, 0x9F, 0xAF, 0xBF, 0xCF, 0xDF, 0xEF, 0xFF, 0x01, 0x22, 0x11, 0x44, 0x33, 0x66, 0x55, 0x79,
    0x29, 0xED, 0xFF, 0xB1, 0xCB, 0x74, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
    0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x66, 0x6F, 0x72, 0x74, 0x72, 0x65, 0x73, 0x73, 0x01, 0x18, 0x17, 0x16, 0x15,
    0x14, 0x13, 0x12, 0x11, 0x76, 0xF3, 0x01, 0x00, 0x01, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x28, 0x27,
    0x26, 0x25, 0x24, 0x23, 0x22, 0x21, 0x02, 0x00, 0x00, 0x00, 0x7F,
];
const REPLAY_BYTES: &[u8] = &[
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x34, 0x12, 0x2E, 0xFB, 0xAA, 0x55, 0x7F, 0x01,
    0xEF, 0xCD, 0xAB, 0x89, 0x01, 0x00, 0x00, 0x00, 0xCD, 0xAB, 0xE1, 0x10, 0x01, 0x02, 0x03, 0x00,
    0x40, 0x30, 0x20, 0x10, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xCD, 0xAB, 0xE1, 0x10, 0x01, 0x02, 0x03, 0x00, 0x40, 0x30, 0x20, 0x10, 0x00, 0x00, 0x00, 0x00,
    0x34, 0x12, 0x2E, 0xFB, 0xAA, 0x55, 0x7F, 0x01, 0xEF, 0xCD, 0xAB, 0x89, 0x01, 0x00, 0x00, 0x00,
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x40, 0x3F, 0x3E, 0x3D, 0x3C, 0x3B,
    0x3A, 0x39, 0x38, 0x37, 0x36, 0x35, 0x34, 0x33, 0x32, 0x31, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x62, 0x69, 0x6E, 0x63, 0x6F, 0x64, 0x65, 0x2D, 0x32, 0x2E, 0x30, 0x2E, 0x31, 0x02,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
const STATE_FNV1A: u128 = 14_907_630_837_206_438_223;
const STATE_FLETCHER16: u128 = 52_403;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
enum InputMode {
    #[default]
    Idle,
    Active,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct CanonicalInput {
    buttons: u16,
    axis: i16,
    flags: [u8; 3],
    active: bool,
    sequence: u32,
    mode: InputMode,
}

const fn input_a() -> CanonicalInput {
    CanonicalInput {
        buttons: 0x1234,
        axis: -1234,
        flags: [0xAA, 0x55, 0x7F],
        active: true,
        sequence: 0x89AB_CDEF,
        mode: InputMode::Active,
    }
}

const fn input_b() -> CanonicalInput {
    CanonicalInput {
        buttons: 0xABCD,
        axis: 4321,
        flags: [1, 2, 3],
        active: false,
        sequence: 0x1020_3040,
        mode: InputMode::Idle,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct NestedState {
    tick: u64,
    delta: i16,
    flags: [bool; 3],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Phase {
    Warmup,
    Active { round: u32, seed: u64 },
    Finished(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct CanonicalState {
    epoch: usize,
    counter: u64,
    signed: i64,
    alive: bool,
    palette: [u16; 3],
    position: (i32, i32),
    players: Vec<u32>,
    label: String,
    checkpoints: [Option<NestedState>; 2],
    phases: Vec<Phase>,
}

fn canonical_state() -> CanonicalState {
    CanonicalState {
        epoch: 0x0102_0304,
        counter: 0x0102_0304_0506_0708,
        signed: -0x0010_2030_4050_6070,
        alive: true,
        palette: [0x1122, 0x3344, 0x5566],
        position: (-1_234_567, 7_654_321),
        players: vec![3, 5, 8],
        label: "fortress".to_owned(),
        checkpoints: [
            Some(NestedState {
                tick: 0x1112_1314_1516_1718,
                delta: -3210,
                flags: [true, false, true],
            }),
            None,
        ],
        phases: vec![
            Phase::Warmup,
            Phase::Active {
                round: 9,
                seed: 0x2122_2324_2526_2728,
            },
            Phase::Finished(0x7F),
        ],
    }
}

#[cfg(feature = "hot-join")]
#[derive(Debug)]
struct GoldenConfig;

#[cfg(feature = "hot-join")]
impl Config for GoldenConfig {
    type Input = CanonicalInput;
    type State = CanonicalState;
    type Address = u8;
}

fn assert_codec_fixture<T>(value: &T, expected: &[u8]) -> Result<(), Box<dyn std::error::Error>>
where
    T: Serialize + DeserializeOwned + Debug + PartialEq,
{
    assert_eq!(codec::encode(value)?, expected);
    assert_eq!(codec::encoded_len(value)?, expected.len());

    let mut fixed = vec![0xA5; expected.len()];
    let written = codec::encode_into(value, &mut fixed)?;
    fixed.truncate(written);
    assert_eq!(fixed, expected);

    let mut appended = vec![0xA5];
    assert_eq!(codec::encode_append(value, &mut appended)?, expected.len());
    assert_eq!(appended.get(1..), Some(expected));

    let (generic, consumed) = codec::decode::<T>(expected)?;
    assert_eq!(generic, *value);
    assert_eq!(consumed, expected.len());

    let (bounded, consumed) = codec::decode_bounded_with_consumed::<T>(expected)?;
    assert_eq!(bounded, *value);
    assert_eq!(consumed, expected.len());
    #[cfg(feature = "hot-join")]
    assert_eq!(codec::decode_bounded::<T>(expected)?, *value);
    Ok(())
}

fn canonical_replay() -> Replay<CanonicalInput> {
    Replay {
        num_players: 2,
        frames: vec![vec![input_a(), input_b()], vec![input_b(), input_a()]],
        checksums: vec![None, Some(0x3132_3334_3536_3738_393A_3B3C_3D3E_3F40)],
        metadata: ReplayMetadata {
            library_version: format!("bincode-{BINCODE_GOLDEN_VERSION}"),
            num_players: 2,
            total_frames: 2,
            skipped_frames: 0,
        },
    }
}

#[test]
fn serialization_golden_fixed_width_inputs_are_immutable() -> Result<(), Box<dyn std::error::Error>>
{
    assert_codec_fixture(&input_a(), INPUT_A_BYTES)?;
    assert_codec_fixture(&input_b(), INPUT_B_BYTES)
}

#[test]
fn serialization_golden_rich_state_is_immutable() -> Result<(), Box<dyn std::error::Error>> {
    assert_codec_fixture(&canonical_state(), STATE_BYTES)
}

#[test]
fn serialization_golden_replay_envelope_is_immutable() -> Result<(), Box<dyn std::error::Error>> {
    let replay = canonical_replay();
    assert_eq!(replay.to_bytes()?, REPLAY_BYTES);
    assert_eq!(Replay::<CanonicalInput>::from_bytes(REPLAY_BYTES)?, replay);
    Ok(())
}

#[test]
fn serialization_golden_state_checksums_are_immutable() -> Result<(), Box<dyn std::error::Error>> {
    let state = canonical_state();
    assert_eq!(hash_bytes_fnv1a(STATE_BYTES), STATE_FNV1A);
    assert_eq!(fletcher16(STATE_BYTES), STATE_FLETCHER16 as u16);
    assert_eq!(compute_checksum(&state)?, STATE_FNV1A);
    assert_eq!(compute_checksum_fletcher16(&state)?, STATE_FLETCHER16);
    Ok(())
}

#[cfg(feature = "hot-join")]
#[test]
fn serialization_golden_hot_join_payloads_share_the_frozen_codec(
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::sessions::hot_join::{
        decode_bridge_inputs, deserialize_state, encode_bridge_inputs, serialize_state,
    };

    let state = canonical_state();
    assert_eq!(serialize_state::<GoldenConfig>(&state)?, STATE_BYTES);
    assert_eq!(deserialize_state::<GoldenConfig>(STATE_BYTES)?, state);

    let bridge = [
        PlayerInput::new(Frame::new(17), input_a()),
        PlayerInput::new(Frame::new(17), input_b()),
    ];
    let mut expected_bridge = INPUT_A_BYTES.to_vec();
    expected_bridge.extend_from_slice(INPUT_B_BYTES);
    assert_eq!(
        encode_bridge_inputs::<GoldenConfig>(&bridge)?,
        expected_bridge
    );
    assert_eq!(
        decode_bridge_inputs::<GoldenConfig>(&expected_bridge, bridge.len())?,
        vec![input_a(), input_b()]
    );
    Ok(())
}
