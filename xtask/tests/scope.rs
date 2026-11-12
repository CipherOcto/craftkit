//! The **scope of the gate**, and the report it makes about it.
//!
//! The gate checks the packages a change can break when it runs on a desk,
//! and the whole tree when it runs on a build machine. The rule is short and
//! it decides what is checked, so it is pinned here — together with the
//! report, because a scoped check that does not say what it left out reads
//! as a whole check.
//!
//! These tests are about the *decision*. Running the gate is the gate's
//! business, and the gate is slow on purpose.

use std::path::PathBuf;

use xtask::ci::{choose_scope_in, the_skipped_line};
use xtask::verify::{Resolution, Scope};

/// The scope of a run on a desk: the blast radius of the change.
#[test]
fn a_desk_checks_the_blast_radius() {
    assert_eq!(choose_scope_in(false, false), Scope::Changed);
}

/// A build machine checks the whole tree.
#[test]
fn a_build_machine_checks_the_whole_tree() {
    assert_eq!(choose_scope_in(false, true), Scope::All);
}

/// The flag asks for the whole tree, wherever the run happens.
#[test]
fn the_flag_asks_for_the_whole_tree() {
    assert_eq!(choose_scope_in(true, false), Scope::All);
    assert_eq!(choose_scope_in(true, true), Scope::All);
}

/// A whole-tree run has nothing to report about what it left out.
#[test]
fn a_whole_tree_run_reports_nothing_about_what_it_left_out() {
    let resolution = Resolution { selected: vec!["a".to_string(), "b".to_string()], skipped: Vec::new() };
    assert!(the_skipped_line(&resolution).is_none(), "nothing was left out, so nothing is said");
}

/// A scoped run names the packages it did not check.
#[test]
fn a_scoped_run_names_the_packages_it_left_out() {
    let resolution = Resolution {
        selected: vec!["craftkit-plugin-grid".to_string()],
        skipped: vec!["craftkit-plugin-sound".to_string(), "craftkit-sdk".to_string()],
    };
    let line = the_skipped_line(&resolution).expect("something was left out");
    assert!(line.contains("1 of 3 packages"), "the line says how much was checked: {line}");
    assert!(line.contains("craftkit-plugin-sound"), "the line names what was left out: {line}");
    assert!(line.contains("craftkit-sdk"), "the line names what was left out: {line}");
}

/// **The build machine checks the whole tree**, and the workflow says so.
///
/// The rule is one line of Rust and the workflow is the other half of it: a
/// rule that the workflow does not follow is a rule that is true on a desk
/// and false in CI, which is the worst place for a rule to be false.
#[test]
fn the_workflow_checks_the_whole_tree() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/ci.yml")).expect("the workflow is there");
    assert!(
        workflow.contains("cargo xtask ci --all"),
        "the workflow runs the gate on the whole tree, and says so"
    );
}
