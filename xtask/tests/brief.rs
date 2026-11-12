//! The **brief**: everything one arc needs, in one page.
//!
//! An arc needs six things before it writes a line — the rule, the landing
//! site, the conventions, the claims, the counts and the commands — and each
//! of them was a search. This is the test that the brief still has all of
//! them: a brief that quietly loses a part is a brief that sends the arc
//! back to searching.

use std::path::{Path, PathBuf};

use xtask::brief;
use xtask::index;

const BATTERIES: [&str; 9] = ["grid", "prose", "pdf", "film", "layout", "cad", "fx", "sound", "vector"];

/// The root of the tree.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().expect("the root is there")
}

/// The README of the tree.
fn readme() -> String {
    std::fs::read_to_string(root().join("README.md")).expect("the README is there")
}

#[test]
fn the_brief_has_every_part() {
    let index = index::build(&root()).expect("the index is built");
    let brief = brief::build(&root(), &index, "grid", "grid.workbook.add").expect("the brief is built");
    assert_eq!(brief.battery, "grid");
    assert_eq!(brief.id, "grid.workbook.add");
    // The landing site.
    assert!(brief.landing.is_some(), "the brief says where the command is: {:?}", brief.landing);
    // The conventions.
    assert!(brief.example.is_some(), "the brief names the battery's example");
    assert!(brief.test.is_some(), "the brief names a test of the battery");
    assert!(!brief.neighbours.is_empty(), "the brief names the battery's other commands");
    // The claims.
    assert!(brief.section.starts_with('§'), "the brief names the section: {}", brief.section);
    assert_eq!(brief.counts.len(), BATTERIES.len(), "the brief carries the counts of the tree");
}

#[test]
fn the_section_is_the_one_the_readme_is_about_to_have() {
    let readme = readme();
    let section = brief::next_section(&readme);
    let number: usize = section.trim_start_matches('§').parse().expect("a number");
    // The highest the README has, plus one.
    let mut highest = 0usize;
    for line in readme.lines() {
        let Some(rest) = line.strip_prefix("## ") else { continue };
        let Some((n, _)) = rest.split_once('.') else { continue };
        let Ok(n) = n.trim().parse::<usize>() else { continue };
        highest = highest.max(n);
    }
    assert_eq!(number, highest + 1, "the next section is the one after the highest");
}

#[test]
fn a_command_that_is_not_in_the_tree_is_not_given_a_landing_site() {
    let index = index::build(&root()).expect("the index is built");
    let brief = brief::build(&root(), &index, "grid", "grid.not.a.command").expect("the brief is built");
    assert!(
        brief.landing.is_none(),
        "a command that is not in the tree has no landing site, and the brief does not invent one"
    );
}

#[test]
fn the_counts_of_the_brief_are_the_counts_of_the_tree() {
    let index = index::build(&root()).expect("the index is built");
    let brief = brief::build(&root(), &index, "grid", "grid.workbook.add").expect("the brief is built");
    for (battery, count) in &brief.counts {
        let in_the_index = index::commands_of(&index, battery).len();
        assert_eq!(in_the_index, *count, "the {battery} battery carries {count} commands");
    }
}
