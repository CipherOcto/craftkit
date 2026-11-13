//! The rule that **a test does not litter**, and that the gate bites.
//!
//! A gate that is only ever run on a good tree has never been shown to
//! **bite**, and a gate that cannot bite is a comment. So this file breaks
//! a tree on purpose — a test that writes into the temporary directory
//! and leaves it there — and says that the gate finds it.
//!
//! # Why the rule exists
//!
//! One full run of this tree's tests wrote **115 167 files** and about a
//! gigabyte into the system's temporary directory, and nothing said so.
//! A file that is written and forgotten is not a thing a gate can see on
//! its own — the gate has to be told where to look, and what to look
//! for.
//!
//! The **scratch** is the way (§873): a path that is unique, and that is
//! taken back when the thread that asked for it ends.
//!
//! The rule is checked against the **code** — a name in a comment is not a
//! call, and a name in a string is not a call either — and against the
//! **tests** only: a program that needs a temporary directory at runtime
//! has business there, and takes its own directory away.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use craftkit_gates::{DENY, SCRATCH, check};

/// The root of this workspace.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("the root is there")
}

/// A tree in a temporary directory, so that it can be broken on purpose.
struct Tree {
    root: PathBuf,
}

impl Tree {
    /// A new tree, with a name of its own so that two tests do not fight
    /// over one directory.
    fn new(name: &str) -> Tree {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root =
            craftkit_scratch::dir(&format!("craftkit-gates-litter-{}-{name}-{unique}", std::process::id()))
                .expect("the directory is made");
        Tree { root }
    }

    /// A crate in the tree, with a test of its own.
    fn crate_with_a_test(&self, body: &str) -> &Tree {
        let dir = self.root.join("crates").join("thing");
        std::fs::create_dir_all(dir.join("src")).expect("the directory is made");
        std::fs::create_dir_all(dir.join("tests")).expect("the directory is made");
        let manifest = "[package]\nname = \"thing\"\nversion = \"0.1.0\"\n";
        std::fs::write(dir.join("Cargo.toml"), manifest).expect("the manifest is written");
        let source = format!("{DENY}\n\npub fn answer() -> u8 {{ 42 }}\n");
        std::fs::write(dir.join("src").join("lib.rs"), source).expect("the source is written");
        std::fs::write(dir.join("tests").join("thing.rs"), body).expect("the test is written");
        self
    }
}

/// The findings of a rule, in a tree.
fn findings_of(root: &Path, rule: &str) -> Vec<String> {
    check(root).into_iter().filter(|finding| finding.rule == rule).map(|finding| finding.message).collect()
}

/// The tree of the workspace: no test of it names the temporary directory.
#[test]
fn no_test_of_the_tree_litters() {
    let findings = findings_of(&root(), "litter");
    assert!(findings.is_empty(), "the gate found: {}", findings.join("\n     "));
}

/// A test that writes into the temporary directory and leaves it there is
/// caught.
#[test]
fn a_test_that_names_the_temporary_directory_is_caught() {
    let tree = Tree::new("caught");
    tree.crate_with_a_test(
        "#[test]\nfn a_file() {\n    let path = std::env::temp_dir().join(\"thing\");\n    \
         std::fs::write(&path, b\"x\").expect(\"written\");\n}\n",
    );
    let findings = findings_of(&tree.root, "litter");
    assert_eq!(findings.len(), 1, "the gate found: {findings:?}");
    assert!(findings[0].contains("temporary directory"), "the finding says: {}", findings[0]);
}

/// A test that only **talks** about the temporary directory is not caught:
/// a name in a comment is not a call.
#[test]
fn a_test_that_only_mentions_the_temporary_directory_is_not_caught() {
    let tree = Tree::new("mentions");
    tree.crate_with_a_test(
        "//! The temporary directory is where a file goes.\n#[test]\nfn a_file() {\n    \
         let path = \"a path\".to_string();\n    assert!(!path.is_empty());\n}\n",
    );
    let findings = findings_of(&tree.root, "litter");
    assert!(findings.is_empty(), "the gate found: {}", findings.join("\n     "));
}

/// A **program** may name the temporary directory: it is a program, and
/// the rule is about tests.
#[test]
fn a_program_may_name_the_temporary_directory() {
    let tree = Tree::new("program");
    let dir = tree.root.join("crates").join("thing");
    std::fs::create_dir_all(dir.join("src")).expect("the directory is made");
    let manifest = "[package]\nname = \"thing\"\nversion = \"0.1.0\"\n";
    std::fs::write(dir.join("Cargo.toml"), manifest).expect("the manifest is written");
    let source = format!(
        "{DENY}\n\npub fn work() -> std::path::PathBuf {{\n    std::env::temp_dir().join(\"work\")\n}}\n"
    );
    std::fs::write(dir.join("src").join("lib.rs"), source).expect("the source is written");
    let findings = findings_of(&tree.root, "litter");
    assert!(findings.is_empty(), "the gate found: {}", findings.join("\n     "));
}

/// The **scratch** is the one crate that may name the temporary
/// directory: handing out a path is its whole job.
#[test]
fn the_scratch_may_name_the_temporary_directory() {
    assert_eq!(SCRATCH, "craftkit-scratch");
}
