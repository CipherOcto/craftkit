//! The **browser door**, driven the way the page drives it.
//!
//! The page writes a command into the module's memory, calls the
//! exports and reads the answer back out of the memory. The exports
//! are the boundary, and they are driven here exactly as the page
//! drives them: the same four calls, in the same order.
//!
//! The one thing the page does that Rust cannot is **write into the
//! module's memory**: a Rust caller writes into the same buffer
//! through the door's own safe door, and the address the page would
//! have used is the address the door gives.

use craftkit_wasm::exports::{
    craftkit_wasm_request_address, craftkit_wasm_response_address, craftkit_wasm_response_length,
    craftkit_wasm_run,
};

/// Ask the door something, the way the page does.
///
/// The steps are the page's: make the room, write the command into it,
/// run it, and read the answer out.
fn ask(command: &str) -> String {
    let bytes = command.as_bytes();
    let address = craftkit_wasm_request_address(bytes.len() as u32);
    assert!(address > 0, "the room for the command has an address");
    craftkit_wasm::write_request(bytes).expect("the room is as big as the command");
    let length = craftkit_wasm_run();
    let where_ = craftkit_wasm_response_address();
    assert!(where_ > 0, "the answer has an address");
    assert_eq!(length, craftkit_wasm_response_length(), "the answer is as long as it says");
    craftkit_wasm::response()
}

/// The commands the door offers.
#[test]
fn the_door_lists_its_commands() {
    let answer = ask("commands");
    assert!(answer.contains("doc.plan"), "the plan is a command: {answer}");
    assert!(answer.contains("session.memory"), "the memory is a command: {answer}");
}

/// The door is the **terminal door**, so the answers are the terminal
/// door's answers.
#[test]
fn the_door_is_the_terminal_door() {
    let terminal = craftkit_wasm::submit("commands");
    let page = ask("commands");
    assert_eq!(terminal, page, "the page and the prompt say the same thing");
}

/// A command runs, and the answer comes back.
#[test]
fn a_command_runs_and_the_answer_comes_back() {
    let answer = ask("help");
    assert!(answer.contains("doc.plan"), "the help lists the commands: {answer}");
}

/// The plan is a command, so the page can ask for it.
#[test]
fn the_plan_is_a_command_the_page_can_ask_for() {
    let answer = ask("doc.plan");
    assert!(!answer.is_empty(), "the plan says something: {answer}");
}

/// An unknown command is refused, and the refusal is an answer.
#[test]
fn an_unknown_command_is_refused() {
    let answer = ask("no.such.command");
    assert!(!answer.is_empty(), "there is an answer");
    assert_ne!(answer, ask("commands"), "the refusal is not the list of commands");
}

/// A command over the bound is refused, and it says so.
#[test]
fn a_command_over_the_bound_is_refused() {
    let long = "x".repeat(64 * 1024 + 1);
    let answer = craftkit_wasm::submit(&long);
    assert!(answer.contains("refused"), "the answer refuses it: {answer}");
}

/// The room for a command is exactly as long as the command.
#[test]
fn the_room_for_a_command_is_the_length_of_the_command() {
    let address = craftkit_wasm::reserve_request(10);
    assert!(address > 0, "the room has an address");
    craftkit_wasm::write_request(b"0123456789").expect("the room is big enough");
    let answer = ask("commands");
    assert!(!answer.is_empty(), "the answer is there");
}
