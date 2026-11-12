//! The **port manifest**: the tree's record of where each rule came from.
//!
//! A port is invisible unless something records it. The manifest is that
//! record, and the gate checks it against the tree: a row that names a
//! command that was renamed away, or a test that was deleted, is a row that
//! has rotted.
//!
//! These tests are the check, and they are written against the tree itself
//! — and against a **broken row**, because a gate that cannot fail is a gate
//! that is not there.

use std::path::{Path, PathBuf};

use xtask::index;
use xtask::ports::{PORTS, Port, problems, problems_of};

/// The root of the tree.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().expect("the root is there")
}

#[test]
fn every_port_is_in_the_tree() {
    let index = index::build(&root()).expect("the index is built");
    let problems = problems(&root(), &index);
    assert!(problems.is_empty(), "the manifest has problems: {problems:?}");
}

#[test]
fn a_port_that_names_a_command_that_is_not_there_is_a_problem() {
    let index = index::build(&root()).expect("the index is built");
    let broken = Port {
        rule: "a rule that was never ported",
        from: "nowhere",
        to: "xtask",
        module: "src/verify.rs",
        command: "xtask.not_a_command",
        test: "xtask/tests/verify.rs",
        section: "§0",
    };
    let problems = problems_of(&root(), &index, &[broken]);
    assert!(
        problems.iter().any(|p| p.contains("xtask.not_a_command")),
        "the broken row is reported: {problems:?}"
    );
}

#[test]
fn a_port_that_names_a_test_that_is_not_there_is_a_problem() {
    let index = index::build(&root()).expect("the index is built");
    let broken = Port {
        rule: "a rule whose test was deleted",
        from: "nowhere",
        to: "xtask",
        module: "src/verify.rs",
        command: "xtask.verify",
        test: "xtask/tests/not_a_test.rs",
        section: "§0",
    };
    let problems = problems_of(&root(), &index, &[broken]);
    assert!(problems.iter().any(|p| p.contains("not_a_test.rs")), "the broken row is reported: {problems:?}");
}

#[test]
fn a_port_that_names_a_module_that_is_not_there_is_a_problem() {
    let index = index::build(&root()).expect("the index is built");
    let broken = Port {
        rule: "a rule whose module was moved",
        from: "nowhere",
        to: "xtask",
        module: "src/not_a_module.rs",
        command: "xtask.verify",
        test: "xtask/tests/verify.rs",
        section: "§0",
    };
    let problems = problems_of(&root(), &index, &[broken]);
    assert!(
        problems.iter().any(|p| p.contains("not_a_module.rs")),
        "the broken row is reported: {problems:?}"
    );
}

#[test]
fn the_manifest_grows_forwards() {
    // The rows are added as the ports are made, so they are in the order they
    // were made. A row that is out of order is a row that was written
    // backwards — and the manifest is forward only.
    let sections: Vec<&str> = PORTS.iter().map(|port| port.section).collect();
    let mut sorted = sections.clone();
    sorted.sort_by_key(|section| section.trim_start_matches('§').parse::<usize>().unwrap_or(0));
    assert_eq!(sections, sorted, "the rows are in the order they were made");
}
