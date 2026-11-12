//! The **delegation rule**, and the gate that keeps it in the prose.
//!
//! The rule is one sentence: **a sub-agent gets a bounded, mechanical brief,
//! or it does not get spawned.** It is a rule about how the tree is worked
//! on, and it decays like every other rule — the sentence that was written
//! once is forgotten, and the tree goes back to spawning a survey for every
//! question.
//!
//! So the rule is in the tree's prose, and the gate checks that the prose
//! still says it. These tests are that check, and they are written against
//! the tree itself rather than against a copy of it.

use std::path::{Path, PathBuf};

/// The root of the tree.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().expect("the root is there")
}

/// The rule, as the prose states it.
const RULE: &str = "bounded, mechanical brief";

#[test]
fn the_playbook_states_the_rule() {
    let text = std::fs::read_to_string(root().join("docs/agent-playbook.md")).expect("the playbook is there");
    assert!(text.contains(RULE), "the playbook does not state the delegation rule");
}

#[test]
fn the_house_standard_states_the_rule() {
    let text = std::fs::read_to_string(root().join("AGENTS.md")).expect("the standard is there");
    assert!(text.contains(RULE), "AGENTS.md does not state the delegation rule");
}

#[test]
fn the_gate_is_happy_with_the_tree() {
    let problems = xtask::lists::the_delegation_rule_is_stated(&root()).expect("the prose can be read");
    assert!(problems.is_empty(), "the gate found: {problems:?}");
}

#[test]
fn the_gate_notices_when_the_rule_is_gone() {
    // A gate that cannot fail is a gate that is not there. So the rule is
    // taken **out** of a copy of the tree, and the gate is asked about that
    // copy: it must complain.
    let copy = copy_of_the_tree();
    let text = std::fs::read_to_string(copy.join("AGENTS.md")).expect("the copy has the standard");
    let without = text.replace(RULE, "a brief of some kind");
    std::fs::write(copy.join("AGENTS.md"), without).expect("the copy can be written");
    let problems = xtask::lists::the_delegation_rule_is_stated(&copy).expect("the copy can be read");
    assert!(!problems.is_empty(), "the gate did not notice that the rule was gone");
}

/// A copy of the tree's prose, in a temporary directory.
///
/// **A counter, not the process id alone.** The tests of one binary run in
/// parallel threads, and two calls that share a folder race: one removes the
/// folder the other is writing into. §843.
fn copy_of_the_tree() -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = root();
    let target = std::env::temp_dir().join(format!("craftkit-delegation-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&target);
    std::fs::create_dir_all(target.join("docs")).expect("the directory is made");
    for file in ["AGENTS.md", "docs/agent-playbook.md"] {
        std::fs::copy(root.join(file), target.join(file)).expect("the file is copied");
    }
    target
}
