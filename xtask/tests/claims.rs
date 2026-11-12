//! The **claim table** as an index.
//!
//! The table of §48 is prose, and prose stays prose — but two of its three
//! columns can be made true by machine: the **where** column must point at
//! sections that exist, and every arc the tree made must have a row.
//!
//! These tests are that check, and the last one is the one that matters: a
//! rule that cannot fail is a rule that is not there.

use std::path::{Path, PathBuf};

use xtask::claims::{problems, rows, sections, sections_in};

/// The root of the tree.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().expect("the root is there")
}

/// The README of the tree.
fn readme() -> String {
    std::fs::read_to_string(root().join("README.md")).expect("the README is there")
}

#[test]
fn the_table_is_found_and_is_a_table() {
    let rows = rows(&readme());
    assert!(rows.len() > 700, "the table has {} rows", rows.len());
    for row in rows.iter().take(5) {
        assert!(!row.claim.is_empty(), "every row says what it claims");
        assert!(!row.truth.is_empty(), "every row says what is true today");
    }
}

#[test]
fn the_numbers_of_a_cell_are_read_out_of_it() {
    assert_eq!(sections_in("§42"), vec![42]);
    assert_eq!(sections_in("§52, §56"), vec![52, 56]);
    assert_eq!(sections_in("§694"), vec![694]);
    assert!(sections_in("none").is_empty(), "a cell with no number names no section");
}

#[test]
fn the_table_of_the_tree_is_an_index() {
    let problems = problems(&readme(), &[]);
    assert!(problems.is_empty(), "the claim table has problems: {problems:?}");
}

#[test]
fn an_arc_with_no_row_is_a_problem() {
    // The rule that bites: a port cannot land without saying what it claims.
    let problems = problems(&readme(), &[9999]);
    assert!(problems.iter().any(|p| p.contains("§9999")), "an arc with no row is reported: {problems:?}");
}

#[test]
fn a_row_that_points_at_nothing_is_a_problem() {
    // The rows of a table are pointers; a pointer that points at nothing is
    // a row that says "somewhere".
    let readme = readme();
    let broken = readme.replace("| §844 |", "| §9998 |");
    let problems = problems(&broken, &[]);
    assert!(
        problems.iter().any(|p| p.contains("§9998")),
        "a row that points at a section that is not there is reported: {problems:?}"
    );
}

#[test]
fn the_sections_of_the_readme_are_read_out_of_it() {
    let sections = sections(&readme());
    assert!(sections.contains(&48), "§48 is the claim table");
    assert!(sections.len() > 800, "the README has {} sections", sections.len());
}
