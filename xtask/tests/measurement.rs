//! The **measurement**, and the procedure that reads it.
//!
//! The plan's exit numbers are session-side: the calls an arc makes, the
//! round trips and the machine time are properties of the *session* that
//! did the work, so they are not in the tree — the runtime's own record
//! holds them. But the **procedure** that reads them against the baseline
//! is in the tree, and a procedure that is not written down is a procedure
//! nobody follows.
//!
//! So the playbook states the rule and names the baseline, and the gate
//! checks that it still does. These tests are that check, written against
//! the tree itself and against a **tree without the procedure**, because a
//! gate that cannot fail is a gate that is not there.

use std::path::{Path, PathBuf};

use xtask::lists;

/// The root of the tree.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().expect("the root is there")
}

/// The playbook of the tree.
fn playbook() -> String {
    std::fs::read_to_string(root().join("docs/agent-playbook.md")).expect("the playbook is there")
}

/// The playbook states the rule: a shortcut that does not move a number did
/// not happen.
#[test]
fn the_playbook_states_the_rule() {
    let text = playbook();
    assert!(
        text.contains("does not move a number"),
        "the playbook does not state the rule of the measurement"
    );
}

/// The playbook names the baseline, and the tree's own half.
#[test]
fn the_playbook_names_the_baseline() {
    let text = playbook();
    for needle in ["35.4", "959", "cargo xtask census"] {
        assert!(text.contains(needle), "the playbook does not name `{needle}`");
    }
}

/// The survey holds the baseline the waves move against.
#[test]
fn the_survey_holds_the_baseline() {
    let text =
        std::fs::read_to_string(root().join("docs/survey-cost-and-meta.md")).expect("the survey is there");
    for needle in ["35.4", "26.8", "959"] {
        assert!(text.contains(needle), "the survey does not hold `{needle}`");
    }
}

/// The gate is happy with the tree.
#[test]
fn the_gate_is_happy_with_the_tree() {
    let problems = lists::the_measurement_procedure_is_stated(&root()).expect("the procedure can be read");
    assert!(problems.is_empty(), "the gate found: {problems:?}");
}

/// A tree whose playbook has lost the procedure is **caught**.
///
/// The check is written against the tree, and the tree is right — so the
/// check has to be shown to be able to fail, and the only way to show that
/// is to take the procedure away and say the gate notices.
#[test]
fn a_playbook_without_the_procedure_is_caught() {
    let directory = xtask::temp::dir("measurement").expect("the folder is there");
    std::fs::create_dir_all(directory.join("docs")).expect("the folder is there");
    // The playbook has lost the procedure; the survey still holds the
    // baseline, so the only thing wrong is the procedure.
    std::fs::write(
        directory.join("docs/agent-playbook.md"),
        "# The agent playbook\n\n## 1. The terminal door\n",
    )
    .expect("the file is written");
    std::fs::write(
        directory.join("docs/survey-cost-and-meta.md"),
        "the calls an arc makes 35.4, the workspace runs 959, the machine time 26.8 h\n",
    )
    .expect("the file is written");
    let problems = lists::the_measurement_procedure_is_stated(&directory).expect("the procedure can be read");
    assert!(!problems.is_empty(), "the gate did not catch the missing procedure");
}
