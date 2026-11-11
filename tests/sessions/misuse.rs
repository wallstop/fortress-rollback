//! Runnable counterexamples for the canonical integration responsibilities.
//!
//! These are deliberately small application-level patterns: a missing rollback
//! save, a checksum serialization failure, and an omitted local input must all
//! fail visibly instead of degrading into silent corruption.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use crate::common::stubs::{StubConfig, StubInput};
use fortress_rollback::{
    compute_checksum, FortressError, Frame, GameStateCell, InvalidRequestKind, PlayerHandle,
    SessionBuilder,
};
use serde::ser::Error as _;
use serde::{Serialize, Serializer};

#[derive(Debug, PartialEq, Eq)]
enum HandlerError {
    MissingSavedState(Frame),
}

fn load_rollback_state(cell: &GameStateCell<u32>, frame: Frame) -> Result<u32, HandlerError> {
    cell.load().ok_or(HandlerError::MissingSavedState(frame))
}

#[test]
fn missing_saved_state_is_a_hard_error() {
    let cell: GameStateCell<u32> = GameStateCell::default();
    let frame = Frame::new(7);
    assert_eq!(
        load_rollback_state(&cell, frame),
        Err(HandlerError::MissingSavedState(frame))
    );

    assert!(cell.save(frame, Some(11), Some(0xAB)));
    assert_eq!(load_rollback_state(&cell, frame), Ok(11));
}

struct UnserializableState;

impl Serialize for UnserializableState {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(S::Error::custom(
            "representative checksum serialization failure",
        ))
    }
}

#[test]
fn checksum_serialization_failure_is_not_erased() {
    let error = compute_checksum(&UnserializableState).expect_err("serialization must fail");
    assert!(
        error.to_string().contains("representative checksum"),
        "the structured checksum error must preserve the serializer cause: {error}"
    );
}

#[test]
fn omitted_local_input_is_rejected_before_advance() {
    let mut session = SessionBuilder::<StubConfig>::new()
        .with_num_players(2)
        .expect("two players")
        .start_synctest_session()
        .expect("session");
    session
        .add_local_input(PlayerHandle::new(0), StubInput { inp: 1 })
        .expect("first local input");

    match session.advance_frame() {
        Err(FortressError::InvalidRequestStructured {
            kind: InvalidRequestKind::MissingLocalInput,
        }) => {},
        other => panic!("omitted local input must return MissingLocalInput, got {other:?}"),
    }
}
