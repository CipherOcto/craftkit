//! The doors, side by side: they agree.
//!
//! The matrix in `docs/door-parity.md` says what each door does, cell by
//! cell, and every cell names the test that proves it. This file is the
//! other half of that claim: the doors are **one kit with six faces**, so
//! what one face says, the others say as well.

use craftkit_parity::{ARGUMENTS, Answer, COMMAND, UNKNOWN, compare, the_session};

/// The first door's answer.
///
/// A test may panic: the report always has an answer, and a report that
/// does not is a failure of the harness and not of a door.
fn first(report: &craftkit_parity::Report) -> &Answer {
    match report.answers.first() {
        Some(answer) => answer,
        None => panic!("a door answered"),
    }
}

/// The doors **agree**.
#[test]
fn the_doors_agree() {
    let report = compare();
    assert!(report.differences.is_empty(), "the doors differ: {:#?}", report.differences);
    assert!(report.answers.len() >= 5, "every door answered: {}", report.answers.len());
}

/// And they agree about **what they list**.
///
/// The list is read from the session the commands run in, and every door
/// reads the same session's shape: the commands of the studio's
/// ecosystem.
#[test]
fn the_doors_list_the_same_commands() {
    let report = compare();
    let first = first(&report);
    // And the list is not empty: two doors that both list nothing would
    // agree about nothing.
    assert!(first.listed.len() > 20, "the doors list the kit's commands: {}", first.listed.len());
    for answer in report.answers.iter().skip(1) {
        let mut mine = answer.listed.clone();
        let mut theirs = first.listed.clone();
        mine.sort();
        theirs.sort();
        assert_eq!(mine, theirs, "{} and {} list different commands", first.door, answer.door);
    }
}

/// And they agree about **what a command says**.
///
/// The same command, run through five doors, has one answer: the
/// command's own message.
#[test]
fn the_doors_say_the_same_thing() {
    let report = compare();
    let first = first(&report);
    for answer in report.answers.iter().skip(1) {
        assert_eq!(answer.ran, first.ran, "{} and {} answered differently", first.door, answer.door);
    }
}

/// And every door **names** the command it refused.
#[test]
fn every_door_names_the_command_it_refused() {
    let report = compare();
    for answer in &report.answers {
        assert!(answer.refused.contains(UNKNOWN), "{} did not name it: {}", answer.door, answer.refused);
    }
}

/// The comparison is about **the kit's commands**, and not about a
/// command that is not there.
#[test]
fn the_command_is_one_the_doors_know() {
    let report = compare();
    let first = first(&report);
    assert!(first.listed.iter().any(|id| id == COMMAND), "{COMMAND} is a command: {:?}", first.listed);
    assert!(!first.listed.iter().any(|id| id == UNKNOWN), "and {UNKNOWN} is not");
    assert!(ARGUMENTS.contains("geometry.rect"), "the arguments are the kind's own");
}

/// And they agree about **the plan**.
///
/// The plan is the **command's** work and not the door's, so five doors
/// asked for it publish the same plan: the same document, read the same
/// way.
#[test]
fn the_doors_publish_the_same_plan() {
    let report = compare();
    let first = first(&report);
    for answer in report.answers.iter().skip(1) {
        assert_eq!(
            answer.planned, first.planned,
            "{} and {} published different plans",
            first.door, answer.door
        );
    }
}

/// And they agree with the **session** — the truth behind them.
///
/// Two doors that are wrong in the same way agree with each other, so
/// the harness asks the session as well: every command a door lists is
/// one the session has, and the answer the door gives, the plan it
/// publishes and the word it says when it undoes are the session's.
#[test]
fn the_doors_agree_with_the_session() {
    let truth = the_session().expect("the session answers");
    let report = compare();
    for answer in &report.answers {
        for id in &answer.listed {
            assert!(
                truth.listed.contains(id),
                "{} lists `{id}`, and the session has no such command",
                answer.door
            );
        }
        assert_eq!(answer.ran, truth.ran, "{} and the session answered differently", answer.door);
        assert_eq!(
            answer.planned, truth.planned,
            "{} and the session published different plans",
            answer.door
        );
        assert_eq!(answer.undone, truth.undone, "{} and the session undid differently", answer.door);
    }
}

/// And they agree about **the undo**.
///
/// The undo is a command like any other, and it says the same thing
/// wherever it runs.
#[test]
fn the_doors_say_the_same_thing_when_they_undo() {
    let report = compare();
    let first = first(&report);
    for answer in report.answers.iter().skip(1) {
        assert_eq!(answer.undone, first.undone, "{} and {} undid differently", first.door, answer.door);
    }
}
