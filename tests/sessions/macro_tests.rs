//! Tests for the `handle_requests!` macro.
//!
//! These tests verify that the macro correctly dispatches to the appropriate
//! handlers and processes requests in order.

// Allow test-specific patterns that are appropriate for test code
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use fortress_rollback::RequestVec;
use fortress_rollback::{handle_requests, Config, FortressRequest, Frame, GameStateCell, InputVec};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::net::SocketAddr;

// Test config for the macro tests
struct MacroTestConfig;

#[derive(Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
struct MacroTestInput(u8);

#[derive(Clone, Default, Debug, PartialEq)]
struct MacroTestState {
    frame: i32,
    value: u64,
}

impl Config for MacroTestConfig {
    type Input = MacroTestInput;
    type State = MacroTestState;
    type Address = SocketAddr;
}

/// Test that the macro compiles with basic usage
#[test]
fn test_handle_requests_compiles() {
    let requests: Vec<FortressRequest<MacroTestConfig>> = vec![];
    let mut state = MacroTestState::default();

    handle_requests!(
        requests,
        save: |_cell: GameStateCell<MacroTestState>, _frame: Frame| {},
        load: |_cell: GameStateCell<MacroTestState>, _frame: Frame| {},
        advance: |_inputs: InputVec<MacroTestInput>| {
            state.frame += 1;
        }
    );

    assert_eq!(state.frame, 0); // No requests, no advances
}

/// Test that advance handler is called correctly
#[test]
fn test_handle_requests_advance() {
    use fortress_rollback::SmallVec;

    let mut state = MacroTestState {
        frame: 0,
        value: 100,
    };

    // Create AdvanceFrame request with test inputs
    let inputs: InputVec<MacroTestInput> = SmallVec::from_vec(vec![
        (MacroTestInput(1), fortress_rollback::InputStatus::Confirmed),
        (MacroTestInput(2), fortress_rollback::InputStatus::Confirmed),
    ]);

    let requests: Vec<FortressRequest<MacroTestConfig>> =
        vec![FortressRequest::AdvanceFrame { inputs }];

    handle_requests!(
        requests,
        save: |_cell: GameStateCell<MacroTestState>, _frame: Frame| {
            panic!("Save should not be called");
        },
        load: |_cell: GameStateCell<MacroTestState>, _frame: Frame| {
            panic!("Load should not be called");
        },
        advance: |inputs: InputVec<MacroTestInput>| {
            state.frame += 1;
            // Sum up input values
            for (input, _status) in inputs.iter() {
                state.value += u64::from(input.0);
            }
        }
    );

    assert_eq!(state.frame, 1);
    assert_eq!(state.value, 103); // 100 + 1 + 2
}

/// Test that multiple advance requests are processed in order
#[test]
fn test_handle_requests_multiple_advances() {
    use fortress_rollback::SmallVec;

    let call_order = RefCell::new(Vec::new());

    let requests: Vec<FortressRequest<MacroTestConfig>> = vec![
        FortressRequest::AdvanceFrame {
            inputs: SmallVec::from_vec(vec![(
                MacroTestInput(1),
                fortress_rollback::InputStatus::Confirmed,
            )]),
        },
        FortressRequest::AdvanceFrame {
            inputs: SmallVec::from_vec(vec![(
                MacroTestInput(2),
                fortress_rollback::InputStatus::Confirmed,
            )]),
        },
        FortressRequest::AdvanceFrame {
            inputs: SmallVec::from_vec(vec![(
                MacroTestInput(3),
                fortress_rollback::InputStatus::Confirmed,
            )]),
        },
    ];

    handle_requests!(
        requests,
        save: |_cell: GameStateCell<MacroTestState>, _frame: Frame| {},
        load: |_cell: GameStateCell<MacroTestState>, _frame: Frame| {},
        advance: |inputs: InputVec<MacroTestInput>| {
            // Record the first input value to verify order
            if let Some((input, _)) = inputs.first() {
                call_order.borrow_mut().push(input.0);
            }
        }
    );

    assert_eq!(*call_order.borrow(), vec![1, 2, 3]);
}

/// Test that the macro works with a trailing comma
#[test]
fn test_handle_requests_trailing_comma() {
    let requests: Vec<FortressRequest<MacroTestConfig>> = vec![];

    handle_requests!(
        requests,
        save: |_cell: GameStateCell<MacroTestState>, _frame: Frame| {},
        load: |_cell: GameStateCell<MacroTestState>, _frame: Frame| {},
        advance: |_inputs: InputVec<MacroTestInput>| {},
    ); // Note the trailing comma

    // If this compiles, the test passes
}

/// Test that the macro works with closures that capture mutable state
#[test]
fn test_handle_requests_mutable_capture() {
    use fortress_rollback::SmallVec;

    let mut save_count = 0u32;
    let mut load_count = 0u32;
    let mut advance_count = 0u32;

    let requests: Vec<FortressRequest<MacroTestConfig>> = vec![
        FortressRequest::AdvanceFrame {
            inputs: SmallVec::new(),
        },
        FortressRequest::AdvanceFrame {
            inputs: SmallVec::new(),
        },
    ];

    handle_requests!(
        requests,
        save: |_cell: GameStateCell<MacroTestState>, _frame: Frame| {
            save_count += 1;
        },
        load: |_cell: GameStateCell<MacroTestState>, _frame: Frame| {
            load_count += 1;
        },
        advance: |_inputs: InputVec<MacroTestInput>| {
            advance_count += 1;
        }
    );

    assert_eq!(save_count, 0);
    assert_eq!(load_count, 0);
    assert_eq!(advance_count, 2);
}

/// Test that the macro works with empty handlers (lockstep mode pattern)
#[test]
fn test_handle_requests_empty_handlers() {
    use fortress_rollback::SmallVec;

    let requests: Vec<FortressRequest<MacroTestConfig>> = vec![FortressRequest::AdvanceFrame {
        inputs: SmallVec::new(),
    }];

    // This is the pattern for lockstep mode where save/load never happen
    handle_requests!(
        requests,
        save: |_, _| {},
        load: |_, _| {},
        advance: |_| {}
    );

    // If this compiles and runs, the test passes
}

/// Test that the macro works with function references instead of closures
#[test]
fn test_handle_requests_function_refs() {
    use fortress_rollback::SmallVec;

    fn on_save(_cell: GameStateCell<MacroTestState>, _frame: Frame) {
        // Save logic
    }

    fn on_load(_cell: GameStateCell<MacroTestState>, _frame: Frame) {
        // Load logic
    }

    fn on_advance(_inputs: InputVec<MacroTestInput>) {
        // Advance logic
    }

    let requests: Vec<FortressRequest<MacroTestConfig>> = vec![FortressRequest::AdvanceFrame {
        inputs: SmallVec::new(),
    }];

    handle_requests!(
        requests,
        save: on_save,
        load: on_load,
        advance: on_advance
    );

    // If this compiles and runs, the test passes
}

/// Test that the macro works with `RequestVec` (the actual return type from `advance_frame`)
#[test]
fn test_handle_requests_with_request_vec() {
    use fortress_rollback::SmallVec;

    let mut advance_count = 0u32;

    // Construct requests using RequestVec, mirroring what advance_frame() returns
    let mut requests: RequestVec<MacroTestConfig> = RequestVec::new();
    requests.push(FortressRequest::AdvanceFrame {
        inputs: SmallVec::new(),
    });
    requests.push(FortressRequest::AdvanceFrame {
        inputs: SmallVec::new(),
    });

    handle_requests!(
        requests,
        save: |_cell: GameStateCell<MacroTestState>, _frame: Frame| {},
        load: |_cell: GameStateCell<MacroTestState>, _frame: Frame| {},
        advance: |_inputs: InputVec<MacroTestInput>| {
            advance_count += 1;
        }
    );

    assert_eq!(advance_count, 2);
}
