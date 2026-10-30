//! The gate: **one** piece of `unsafe` in v2, and it is this one.
//!
//! The exception is allowed because it is measurable and small: a
//! counting global allocator, so that the lane's "must not allocate"
//! rule can be *measured* rather than asserted. The price of allowing
//! it is that the tree now has a place where the rule can be broken, and
//! this gate is what holds that place to one crate.
//!
//! What it checks:
//!
//! - **this crate is the only one with `unsafe` in it** — no `unsafe
//!   impl`, no `unsafe fn`, no `unsafe` block, no `unsafe` trait, and no
//!   `allow(unsafe_code)` anywhere else in the tree;
//! - **this crate has exactly one exception**, and it is on the
//!   allocator's impl;
//! - **this crate says `deny(unsafe_code)` and not `forbid`**, because a
//!   `forbid` could not be relaxed by the one `allow` it needs;
//! - **every other crate still forbids unsafe outright**.

use std::path::{Path, PathBuf};

/// The root of the workspace.
fn workspace_root() -> PathBuf {
    // `<root>/crates/craftkit-alloc` — two levels up is the root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .expect("the crate is inside the workspace")
}

/// Every Rust file in the workspace, with the crate it belongs to.
fn every_source() -> Vec<(String, PathBuf)> {
    let root = workspace_root();
    let mut found = Vec::new();
    for directory in ["crates", "apps"] {
        let Ok(entries) = std::fs::read_dir(root.join(directory)) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
            collect(&path.join("src"), &name, &mut found);
            collect(&path.join("tests"), &name, &mut found);
        }
    }
    found
}

/// Every Rust file under a directory.
fn collect(directory: &Path, crate_name: &str, found: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(directory) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, crate_name, found);
            continue;
        }
        if path.extension().map(|extension| extension == "rs").unwrap_or(false) {
            found.push((crate_name.to_string(), path));
        }
    }
}

/// The lines of a file that carry one of the marks.
///
/// The **comments are not code**: this crate's own documentation talks
/// about `unsafe impl` and `unsafe trait`, and a gate that read the
/// prose would count the prose.
fn lines_with(source: &str, marks: &[&str]) -> Vec<String> {
    source
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.starts_with("//"))
        .filter(|line| marks.iter().any(|mark| line.contains(mark)))
        .map(|line| line.to_string())
        .collect()
}

/// The marks that mean "there is unsafe in this file".
fn unsafe_marks() -> Vec<&'static str> {
    vec!["unsafe impl", "unsafe fn", "unsafe trait", "unsafe {", "allow(unsafe_code)"]
}

/// Only this crate has unsafe in it.
#[test]
fn only_this_crate_has_unsafe_in_it() {
    let offenders: Vec<String> = every_source()
        .into_iter()
        .filter(|(crate_name, _)| crate_name != "craftkit-alloc")
        .filter_map(|(crate_name, path)| {
            let source = std::fs::read_to_string(&path).ok()?;
            let lines = lines_with(&source, &unsafe_marks());
            if lines.is_empty() {
                return None;
            }
            Some(format!("{crate_name}: {path:?}: {lines:?}"))
        })
        .collect();
    assert!(offenders.is_empty(), "there is unsafe in the tree: {offenders:#?}");
}

/// This crate has exactly one exception, and it is the allocator's.
#[test]
fn this_crate_has_exactly_one_exception() {
    let source = include_str!("../src/lib.rs");
    let exceptions = lines_with(source, &["allow(unsafe_code)"]);
    assert_eq!(exceptions.len(), 1, "there is one exception: {exceptions:?}");
    let impls = lines_with(source, &["unsafe impl"]);
    assert_eq!(impls.len(), 1, "there is one unsafe impl: {impls:?}");
    assert!(impls[0].contains("GlobalAlloc for Counting"), "the impl is the allocator's: {impls:?}");
}

/// The exception is inside the impl, and nowhere else.
#[test]
fn the_unsafe_is_inside_the_impl() {
    let source = include_str!("../src/lib.rs");
    let blocks = lines_with(source, &["unsafe {"]);
    // One per method of the impl, and no more.
    assert!(blocks.len() <= 4, "there are unsafe blocks: {blocks:?}");
    for block in &blocks {
        assert!(block.contains("std::alloc::System"), "the block is the allocator's: {block}");
    }
    let traits = lines_with(source, &["unsafe trait"]);
    assert!(traits.is_empty(), "there is an unsafe trait: {traits:?}");
}

/// This crate says `deny`, and not `forbid`.
///
/// A `forbid` could not be relaxed by the one `allow` the allocator
/// needs, so the crate root has to say `deny`. Everything else in the
/// crate is under the same rule as everywhere else.
#[test]
fn this_crate_says_deny_and_not_forbid() {
    let source = include_str!("../src/lib.rs");
    assert!(source.contains("#![deny(unsafe_code)]"), "the crate denies unsafe");
    assert!(
        !source.contains("#![forbid(unsafe_code)]"),
        "the crate does not forbid it: the exception is here"
    );
}

/// Every other crate still forbids unsafe outright.
#[test]
fn every_other_crate_forbids_unsafe() {
    let offenders: Vec<String> = every_source()
        .into_iter()
        .filter(|(crate_name, _)| crate_name != "craftkit-alloc")
        .filter_map(|(crate_name, path)| {
            let source = std::fs::read_to_string(&path).ok()?;
            // The crate root is the file that has the crate's attributes.
            if !source.contains("#![forbid(unsafe_code)]") && source.contains("#![deny(") {
                return Some(format!("{crate_name}: {path:?}"));
            }
            None
        })
        .collect();
    assert!(offenders.is_empty(), "a crate root does not forbid unsafe: {offenders:#?}");
}
