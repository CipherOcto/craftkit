//! The **census**: what an arc costs, measured from the history.
//!
//! The survey measured what the v2 work cost in the session that made it.
//! Those numbers describe a model. The census measures the other half —
//! the half that is a property of the **tree**: how big an arc is, how
//! many files it touches, how long it takes from one arc to the next.
//!
//! These tests are the check, and they are written against the tree's own
//! history — and against a **made-up** log, because a parser that cannot
//! fail is a parser that is not there.

use std::path::{Path, PathBuf};

use xtask::census::{self, Arc};

/// The root of the tree.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().expect("the root is there")
}

/// A log of two commits, in git's own shape.
fn a_log() -> String {
    let mut log = String::new();
    log.push_str("1111111111111111111111111111111111111111\u{1f}1000\u{1f}feat(x): the first — §10\n");
    log.push('\n');
    log.push_str("10\t2\tcrates/a/src/lib.rs\n");
    log.push_str("3\t1\tcrates/a/tests/a.rs\n");
    log.push_str("2222222222222222222222222222222222222222\u{1f}2000\u{1f}docs: the first — §10\n");
    log.push('\n');
    log.push_str("40\t0\tREADME.md\n");
    log
}

/// The log is read as the arcs it holds.
#[test]
fn the_log_is_read_as_arcs() {
    let census = census::parse(&a_log(), None);
    assert_eq!(census.commits, 2, "{census:?}");
    assert_eq!(census.arcs.len(), 2, "{census:?}");
    assert_eq!(census.arcs[0].added, 13, "{census:?}");
    assert_eq!(census.arcs[0].removed, 3, "{census:?}");
    assert_eq!(census.arcs[0].files, 2, "{census:?}");
    assert_eq!(census.arcs[0].size(), 16, "{census:?}");
    assert_eq!(census.arcs[1].added, 40, "{census:?}");
}

/// The § a subject names is found, and a subject without one is none.
#[test]
fn the_section_of_a_subject_is_found() {
    assert_eq!(census::the_section_of("feat(x): a thing — §123"), Some("123".to_string()));
    assert_eq!(census::the_section_of("chore: a thing"), None);
    assert_eq!(census::the_section_of("feat(x): a thing — §"), None);
}

/// The median is the middle, and a list of one is that one.
#[test]
fn the_median_is_the_middle() {
    let mut values = vec![5, 1, 3];
    assert_eq!(census::median(&mut values), Some(3));
    let mut even = vec![1, 2, 3, 4];
    assert_eq!(census::median(&mut even), Some(2));
    let mut one = vec![7];
    assert_eq!(census::median(&mut one), Some(7));
    let mut none: Vec<usize> = Vec::new();
    assert_eq!(census::median(&mut none), None);
}

/// The census of the tree's own history is a census.
#[test]
fn the_census_of_the_tree_works() {
    let census = census::census(&root()).expect("the history is there");
    assert!(census.commits > 100, "the census found {} commits", census.commits);
    assert!(!census.arcs.is_empty(), "the census found no arcs");
    // Every arc that names a section has one that is a number.
    for arc in &census.arcs {
        if let Some(section) = &arc.section {
            assert!(section.chars().all(|c| c.is_ascii_digit()), "{arc:?}");
        }
    }
}

/// The census starts at the v2 tree's own beginning.
#[test]
fn the_census_starts_at_the_v2_tree() {
    let census = census::census(&root()).expect("the history is there");
    assert!(census.since.is_some(), "the census did not find where the v2 tree begins");
}

/// The tree's history has an arc that names a section, and the census
/// finds it.
#[test]
fn an_arc_of_the_history_names_its_section() {
    let census = census::census(&root()).expect("the history is there");
    let named: Vec<&Arc> = census.arcs.iter().filter(|arc| arc.section.is_some()).collect();
    assert!(named.len() > 100, "only {} arcs name a section", named.len());
}

/// A log that is empty is a census of nothing, and not a crash.
#[test]
fn an_empty_log_is_a_census_of_nothing() {
    let census = census::parse("", None);
    assert_eq!(census.commits, 0);
    assert!(census.arcs.is_empty());
}
