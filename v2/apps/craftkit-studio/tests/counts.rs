//! The **counts** this README says are the counts of the tree.
//!
//! The README says how many commands each battery carries, and a
//! number that is wrong is a claim that cannot be believed. This test
//! checks the last count of every battery against the source of that
//! battery, and it checks that the source of the tree is text.

use std::fs;
use std::path::{Path, PathBuf};

/// The **batteries** this README counts, and the crate of each.
const BATTERIES: [&str; 9] = ["grid", "prose", "pdf", "film", "layout", "cad", "fx", "sound", "vector"];

/// The **root** of the tree: the folder that holds the crates and this
/// README.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("the root is there")
}

/// The **README** of the tree.
fn readme(root: &Path) -> String {
    fs::read_to_string(root.join("README.md")).expect("the README is there")
}

/// The **source** of a crate: every file of it that is Rust.
fn source_of(root: &Path, crate_name: &str) -> Vec<PathBuf> {
    let directory = root.join("crates").join(format!("craftkit-plugin-{crate_name}")).join("src");
    let mut files = Vec::new();
    let entries = fs::read_dir(&directory).expect("the crate is there");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

/// The **commands** a crate has: the attributes that carry an id.
fn commands_of(root: &Path, crate_name: &str) -> usize {
    let mut total = 0;
    for path in source_of(root, crate_name) {
        let bytes = fs::read(&path).expect("the file is there");
        total += count(&bytes, b"#[command(");
    }
    total
}

/// How many times a **needle** is in a **haystack**.
fn count(haystack: &[u8], needle: &[u8]) -> usize {
    let mut total = 0;
    let mut at = 0;
    while at + needle.len() <= haystack.len() {
        if &haystack[at..at + needle.len()] == needle {
            total += 1;
            at += needle.len();
        } else {
            at += 1;
        }
    }
    total
}

/// The **highest** count the README gives for a battery.
///
/// The commands of a battery are only ever added, so the highest count
/// the README says is the count of the tree.
fn highest_count(readme: &str, battery: &str) -> Option<usize> {
    // The README writes the name of a battery as it likes — `PDF` and
    // `pdf` are the same battery — and the words are read without
    // regard to case.
    let flat: String = readme.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let needle = format!("{} battery ", battery.to_lowercase());
    let mut found = None;
    let mut rest = flat.as_str();
    while let Some(at) = rest.find(&needle) {
        let tail = &rest[at + needle.len()..];
        // The count follows the words, with **now** between them.
        let tail = tail.strip_prefix("now ").unwrap_or(tail);
        if let Some(open) = tail.strip_prefix("carries **") {
            let number = &open[..open.find("**")?];
            if let Ok(count) = number.parse::<usize>() {
                found = Some(found.map_or(count, |held: usize| held.max(count)));
            }
        }
        rest = &rest[at + needle.len()..];
    }
    found
}

/// The count the README gives for a battery is the count of the
/// battery: the highest one it says is the count of the tree.
#[test]
fn the_counts_of_the_readme_are_the_counts_of_the_tree() {
    let root = root();
    let readme = readme(&root);
    for battery in BATTERIES {
        let claimed = highest_count(&readme, battery)
            .unwrap_or_else(|| panic!("the README counts the {battery} battery"));
        let truth = commands_of(&root, battery);
        assert_eq!(
            claimed, truth,
            "the README says the {battery} battery carries {claimed} commands, and it carries {truth}"
        );
    }
}

/// The **source** of the tree is text: no file of it holds a NUL byte,
/// which would make it a file of bytes and not a file of words.
#[test]
fn the_source_of_the_tree_is_text() {
    let root = root();
    for folder in ["crates", "apps"] {
        let mut stack = vec![root.join(folder)];
        while let Some(directory) = stack.pop() {
            let entries = fs::read_dir(&directory).expect("the folder is there");
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                    continue;
                }
                let bytes = fs::read(&path).expect("the file is there");
                assert!(!bytes.contains(&0u8), "the file {} holds a NUL byte", path.display());
            }
        }
    }
}
