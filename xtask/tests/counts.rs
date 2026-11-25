//! The counts the tree states of its own bounds, and the gate that checks
//! them.
//!
//! The README's **counted block** is written by `cargo xtask counts
//! --write`, and the claim table states what is true today. Both of them
//! carry numbers about the tree's caps, and a number that is wrong is a
//! claim that cannot be believed.
//!
//! These tests are about the **readers** of those numbers: the rule that
//! reads the block, the rule that reads the claim table, and the number
//! parser they share.

use std::path::{Path, PathBuf};

use xtask::lists::{is_a_number_word, number_in_words, stated_bounds, the_number_before};

/// The root of the tree.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().expect("the root is there")
}

/// A number in words is a number.
#[test]
fn a_number_in_words_is_a_number() {
    assert_eq!(number_in_words("one"), Some(1));
    assert_eq!(number_in_words("twenty-seven"), Some(27));
    assert_eq!(number_in_words("one hundred and thirty-five"), Some(135));
    assert_eq!(number_in_words("a hundred and forty-four"), Some(144));
    assert_eq!(number_in_words("135"), Some(135));
    assert_eq!(number_in_words("the"), None);
}

/// A token that is made of number words is a number, and the words around
/// it are not.
#[test]
fn a_token_of_number_words_is_a_number() {
    assert!(is_a_number_word("twenty-seven"));
    assert!(is_a_number_word("135"));
    assert!(is_a_number_word("hundred"));
    assert!(!is_a_number_word("the"));
    assert!(!is_a_number_word("**built.**"));
    assert!(!is_a_number_word("caps"));
}

/// The number before a phrase is the number the phrase counts.
#[test]
fn the_number_before_a_phrase_is_the_number() {
    assert_eq!(
        the_number_before(
            "**built.** The hundred and thirty-five caps of the tree are named",
            "caps of the tree"
        ),
        Some(135)
    );
    assert_eq!(
        the_number_before(
            "**built.** Twenty-seven crates keep a published cap",
            "crates keep a published cap"
        ),
        Some(27)
    );
    assert_eq!(the_number_before("nothing here", "caps of the tree"), None);
}

/// The rows of the bounds table are read, and the rows of the commands
/// table are not.
#[test]
fn the_bounds_rows_are_read() {
    let block = "<!-- begin -->\n| battery | commands |\n|---|---|\n| grid | 251 |\n\n| crate | published caps | private caps |\n|---|---|---|\n| craftkit-sdk | 17 | 11 |\n| the tree | 135 | 190 |\n<!-- end -->";
    let rows = stated_bounds(block);
    assert_eq!(rows.len(), 2, "the commands table is not a bounds table: {rows:?}");
    assert_eq!(rows[0], ("craftkit-sdk".to_string(), 17, 11));
    assert_eq!(rows[1], ("the tree".to_string(), 135, 190));
}

/// The tree's block is the tree's counts.
#[test]
fn the_tree_counts_are_true() {
    let problems = xtask::lists::the_bounds_are_counted(&root()).expect("the tree is read");
    assert!(problems.is_empty(), "the counted block is the tree's: {problems:?}");
}

/// The claim table's counts are the tree's counts.
#[test]
fn the_claims_counts_are_true() {
    let problems = xtask::lists::the_claims_counts_are_true(&root()).expect("the tree is read");
    assert!(problems.is_empty(), "the claim table states the tree's counts: {problems:?}");
}

/// The rule bites: a block that says the wrong number is a finding.
#[test]
fn a_block_that_is_wrong_is_a_finding() {
    let bounds = xtask::index::bounds_of(&root());
    let mut right: Vec<(String, usize, usize)> = bounds.by_crate.clone();
    right.push(("the tree".to_string(), bounds.published, bounds.private));
    assert!(
        xtask::lists::bounds_differences(&right, &bounds).is_empty(),
        "a block that is right is not a finding"
    );
    let mut wrong = right.clone();
    for row in wrong.iter_mut() {
        if row.0 == "the tree" {
            row.1 += 1;
        }
    }
    let problems = xtask::lists::bounds_differences(&wrong, &bounds);
    assert!(!problems.is_empty(), "a block that is wrong is a finding: {problems:?}");
}

/// A crate that is missing from the block is a finding, and so is a crate
/// whose numbers are wrong.
#[test]
fn a_crate_that_is_missing_or_wrong_is_a_finding() {
    let bounds = xtask::index::bounds_of(&root());
    let (name, published, private) =
        bounds.by_crate.iter().find(|(_, published, _)| *published > 0).expect("a crate publishes a cap");
    let missing = xtask::lists::bounds_differences(&[], &bounds);
    assert!(!missing.is_empty(), "a crate that is missing is a finding: {missing:?}");
    let wrong = vec![(name.clone(), published + 1, *private)];
    let problems = xtask::lists::bounds_differences(&wrong, &bounds);
    assert!(!problems.is_empty(), "a crate whose numbers are wrong is a finding: {problems:?}");
}
