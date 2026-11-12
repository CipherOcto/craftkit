//! The **digest** of the house: the rules an arc keeps re-learning.
//!
//! A digest that prints a rule the tree no longer has is worse than no
//! digest: it is a rule that has been retired and is still being obeyed.
//! So the house digest **fails** when a rule cannot be found where it says
//! it is.
//!
//! These tests are the check — against the tree itself, and against a
//! **broken tree**, because a gate that cannot fail is a gate that is not
//! there.

use std::path::{Path, PathBuf};

use xtask::digest::{self, Item, SECTION_HEADINGS};

/// The root of the tree.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().expect("the root is there")
}

/// A tree of its own, with the files the digest reads.
fn a_tree(tag: &str) -> PathBuf {
    let directory = xtask::temp::dir(tag).expect("the folder is there");
    std::fs::create_dir_all(directory.join("crates/craftkit-gates/src")).expect("the folders are there");
    std::fs::write(
        directory.join("crates/craftkit-gates/src/lib.rs"),
        "pub const LINTS: &[&str] = &[\n    \"clippy::panic\",\n];\n",
    )
    .expect("the file is written");
    std::fs::write(directory.join("AGENTS.md"), "the rules of the house\n").expect("the file is written");
    std::fs::write(directory.join("CONTRIBUTING.md"), "the rules of the house\n")
        .expect("the file is written");
    std::fs::write(
        directory.join("README.md"),
        "# The tree\n\n## 1. The first section\n\n### What was built\n\n### What the port found\n\n### What it cost\n\n### What it does not settle\n",
    )
    .expect("the file is written");
    directory
}

/// Every rule of the house is in the tree.
#[test]
fn every_rule_of_the_house_is_in_the_tree() {
    let digest = digest::house(&root()).expect("the rules are all there");
    assert!(!digest.is_empty(), "the digest says nothing");
    for title in ["Never crash", "The deny list", "The refusal style"] {
        assert!(digest.contains(title), "the digest does not say {title}");
    }
}

/// The digest says **where** each rule is written.
#[test]
fn the_digest_cites_the_file_of_each_rule() {
    let digest = digest::house(&root()).expect("the rules are all there");
    assert!(digest.contains("CONTRIBUTING.md:"), "the digest cites no file");
    assert!(digest.contains("AGENTS.md:"), "the digest cites no file");
}

/// A rule that has gone is a **failure**.
#[test]
fn a_rule_that_has_gone_is_a_failure() {
    let tree = a_tree("digest-gone");
    let items = [Item::Quoted {
        title: "A rule that was retired",
        file: "AGENTS.md",
        marker: "the rule that is not there any more",
    }];
    let error = digest::house_of(&tree, &items).expect_err("the rule has gone");
    assert!(error.contains("A rule that was retired"), "{error}");
    assert!(error.contains("have gone"), "{error}");
}

/// A rule that is there is not a failure.
#[test]
fn a_rule_that_is_there_is_not_a_failure() {
    let tree = a_tree("digest-there");
    let items =
        [Item::Quoted { title: "A rule that is there", file: "AGENTS.md", marker: "the rules of the house" }];
    let digest = digest::house_of(&tree, &items).expect("the rule is there");
    assert!(digest.contains("A rule that is there"), "{digest}");
}

/// A section without the four headings is a failure.
#[test]
fn a_section_without_the_four_headings_is_a_failure() {
    let tree = a_tree("digest-headings");
    std::fs::write(tree.join("README.md"), "# The tree\n\n## 1. The first section\n\n### What was built\n")
        .expect("the file is written");
    let items = [Item::SectionHeadings];
    let error = digest::house_of(&tree, &items).expect_err("the headings have gone");
    assert!(error.contains("have gone"), "{error}");
}

/// The README's last section has the four headings.
#[test]
fn the_last_section_has_the_four_headings() {
    let readme = std::fs::read_to_string(root().join("README.md")).expect("the README is there");
    let headings = digest::the_last_headings(&readme);
    for heading in SECTION_HEADINGS {
        assert!(
            headings.iter().any(|found| found == heading),
            "the last section has no {heading}: {headings:?}"
        );
    }
}

/// The battery's digest names its crate, its example and its modules.
#[test]
fn the_battery_digest_names_the_battery() {
    let digest = digest::battery(&root(), "grid").expect("the battery is there");
    assert!(digest.contains("craftkit-plugin-grid"), "{digest}");
    assert!(digest.contains("its example is"), "{digest}");
    assert!(digest.contains("its modules:"), "{digest}");
    assert!(digest.contains("its tests:"), "{digest}");
}

/// A battery that is not there is an error, not an empty digest.
#[test]
fn a_battery_that_is_not_there_is_an_error() {
    let error = digest::battery(&root(), "not-a-battery").expect_err("the battery is not there");
    assert!(error.contains("not-a-battery"), "{error}");
}
