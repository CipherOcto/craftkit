//! The gate: **the list** of crates in v2 that may hold `unsafe`, and
//! the proof that the list is short.
//!
//! The exceptions are allowed because they are small, named and
//! mechanical. Each one on the list says what it is for, and the rules
//! are:
//!
//! - `unsafe` appears **only** in the crates on the list;
//! - every crate on the list says `deny(unsafe_code)` and **not**
//!   `forbid`, because a `forbid` could not be relaxed by the `allow` it
//!   needs;
//! - every crate **not** on the list still forbids unsafe outright;
//! - every crate on the list has **exactly one** `allow(unsafe_code)`,
//!   so the exception is one marked place and not a scattering;
//! - the only `unsafe impl` in the tree is the counting allocator's.
//!
//! The list is one list, here, and it is the whole of the exceptions.
//! A sixth exception is a line in this file and a reason in it.

use std::path::{Path, PathBuf};

/// The crates that may hold `unsafe`, and why each one does.
///
/// **This is the list.** Nothing outside it may hold `unsafe`, and the
/// test below reads the whole workspace to say so.
const ALLOWED: &[(&str, &str)] = &[
    ("craftkit-alloc", "the counting allocator: a global allocator is unsafe by definition"),
    ("craftkit-ffi", "the plugin loader: opening a library and calling into it"),
    ("craftkit-plugin-shared", "a shared library's entry point, which is found by name"),
    ("craftkit-clap", "the CLAP host: opening a plugin and reading the entry it exports"),
    ("craftkit-clap-fixture", "a reference CLAP plugin: the entry point is a data symbol, found by name"),
];

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
            // This file is the gate, and it talks about `unsafe` in its
            // own words. A gate that counted itself would count its own
            // prose.
            if path.ends_with("tests/gate.rs") {
                continue;
            }
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
    vec![
        "unsafe impl",
        "unsafe fn",
        "unsafe trait",
        "unsafe {",
        "unsafe extern",
        "unsafe(no_mangle)",
        "allow(unsafe_code)",
    ]
}

/// Whether a crate is on the list.
fn is_allowed(crate_name: &str) -> bool {
    ALLOWED.iter().any(|(name, _)| *name == crate_name)
}

/// Only the crates on the list have unsafe in them.
#[test]
fn only_the_crates_on_the_list_have_unsafe_in_them() {
    let offenders: Vec<String> = every_source()
        .into_iter()
        .filter(|(crate_name, _)| !is_allowed(crate_name))
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

/// Every crate on the list **has** unsafe in it: the list does not carry
/// a name that has moved.
#[test]
fn every_crate_on_the_list_still_needs_its_exception() {
    for (name, reason) in ALLOWED {
        let found = every_source().into_iter().filter(|(crate_name, _)| crate_name == name).count();
        assert!(found > 0, "the crate `{name}` is not in the tree any more: {reason}");
        let unsafe_lines: usize = every_source()
            .into_iter()
            .filter(|(crate_name, _)| crate_name == name)
            .filter_map(|(_, path)| std::fs::read_to_string(&path).ok())
            .map(|source| lines_with(&source, &unsafe_marks()).len())
            .sum();
        assert!(unsafe_lines > 0, "the crate `{name}` has no unsafe in it any more: {reason}");
    }
}

/// Each crate on the list has **exactly one** exception: one marked
/// place, and not a scattering of them.
#[test]
fn each_crate_on_the_list_has_exactly_one_exception() {
    for (name, _) in ALLOWED {
        let exceptions: Vec<String> = every_source()
            .into_iter()
            .filter(|(crate_name, _)| crate_name == *name)
            .filter_map(|(_, path)| std::fs::read_to_string(&path).ok())
            .flat_map(|source| lines_with(&source, &["allow(unsafe_code)"]))
            .collect();
        assert_eq!(exceptions.len(), 1, "the crate `{name}` has one exception: {exceptions:?}");
    }
}

/// The crate on the list that has an `unsafe impl` has **one**, and it
/// is the allocator's.
#[test]
fn the_only_unsafe_impl_is_the_allocators() {
    let mut found: Vec<String> = Vec::new();
    for (crate_name, path) in every_source() {
        let Ok(source) = std::fs::read_to_string(&path) else { continue };
        for line in lines_with(&source, &["unsafe impl"]) {
            found.push(format!("{crate_name}: {line}"));
        }
    }
    assert_eq!(found.len(), 1, "there is one unsafe impl: {found:?}");
    assert!(found[0].contains("craftkit-alloc"), "the impl is the allocator's: {found:?}");
    assert!(found[0].contains("GlobalAlloc for Counting"), "the impl is the allocator's: {found:?}");
}

/// Every crate on the list says `deny`, and not `forbid`.
///
/// A `forbid` could not be relaxed by the `allow` it needs, so the
/// crate roots have to say `deny`.
#[test]
fn the_crates_on_the_list_say_deny_and_not_forbid() {
    for (name, _) in ALLOWED {
        let roots: Vec<PathBuf> = every_source()
            .into_iter()
            .filter(|(crate_name, path)| crate_name == *name && path.ends_with("lib.rs"))
            .map(|(_, path)| path)
            .collect();
        assert!(!roots.is_empty(), "the crate `{name}` has no root");
        for root in roots {
            let source = std::fs::read_to_string(&root).expect("the root is readable");
            assert!(source.contains("#![deny(unsafe_code)]"), "the crate {root:?} denies unsafe");
            assert!(
                !source.contains("#![forbid(unsafe_code)]"),
                "the crate {root:?} does not forbid it: the exception is here"
            );
        }
    }
}

/// Every other crate still forbids unsafe outright.
#[test]
fn every_other_crate_forbids_unsafe() {
    let offenders: Vec<String> = every_source()
        .into_iter()
        .filter(|(crate_name, _)| !is_allowed(crate_name))
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
