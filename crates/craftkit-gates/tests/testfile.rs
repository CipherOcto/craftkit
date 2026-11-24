//! The rule that **every test file is a test the tree runs**, and that the
//! gate bites.
//!
//! A file that looks like a test and is not one is a lie about coverage: a
//! reader counts the test files and believes the count. The tree had one
//! such file — a byte-for-byte copy of a test, in a directory cargo never
//! compiles — and nothing said so, because nothing looked.
//!
//! So this file breaks a tree on purpose and says that the gate finds it.
//! The **scratch** hands out the directory, and the tree's own gate is run
//! over a tree that is broken rather than over the tree that is good.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use craftkit_gates::{DENY, check};

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
            craftkit_scratch::dir(&format!("craftkit-gates-testfile-{}-{name}-{unique}", std::process::id()))
                .expect("the directory is made");
        Tree { root }
    }

    /// A crate in the tree, with a test of its own.
    fn crate_with_a_test(&self, body: &str) -> &Tree {
        self.crate_named("thing", body)
    }

    /// A crate in the tree, with a name of its own and a test of its own.
    fn crate_named(&self, name: &str, body: &str) -> &Tree {
        let dir = self.root.join("crates").join(name);
        std::fs::create_dir_all(dir.join("src")).expect("the directory is made");
        std::fs::create_dir_all(dir.join("tests")).expect("the directory is made");
        let manifest = format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n");
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

/// The tree of the workspace: every test file in it is a test the tree runs.
#[test]
fn every_test_file_of_the_tree_is_a_test_the_tree_runs() {
    let findings = findings_of(&root(), "test-file");
    assert!(findings.is_empty(), "the gate found: {}", findings.join("\n     "));
}

/// A test file in a **directory** under `tests/` is caught: cargo compiles
/// the files that are directly under `tests/`, and not the ones in a
/// directory.
#[test]
fn a_test_file_in_a_directory_under_tests_is_caught() {
    let tree = Tree::new("nested");
    tree.crate_with_a_test("#[test]\nfn a_file() {}\n");
    let nested = tree.root.join("crates").join("thing").join("tests").join("somewhere");
    std::fs::create_dir_all(&nested).expect("the directory is made");
    std::fs::write(nested.join("other.rs"), "#[test]\nfn another() {}\n").expect("the file is written");
    let findings = findings_of(&tree.root, "test-file");
    assert_eq!(findings.len(), 1, "the gate found: {findings:?}");
    assert!(findings[0].contains("never runs"), "the finding says: {}", findings[0]);
}

/// A directory's **`main.rs`** is a test cargo does compile, so it is not a
/// finding: the rule is about the files cargo ignores, and not about the
/// directories.
#[test]
fn a_main_rs_in_a_directory_under_tests_is_not_caught() {
    let tree = Tree::new("main");
    tree.crate_with_a_test("#[test]\nfn a_file() {}\n");
    let nested = tree.root.join("crates").join("thing").join("tests").join("somewhere");
    std::fs::create_dir_all(&nested).expect("the directory is made");
    std::fs::write(nested.join("main.rs"), "#[test]\nfn another() {}\n").expect("the file is written");
    let findings = findings_of(&tree.root, "test-file");
    assert!(findings.is_empty(), "the gate found: {}", findings.join("\n     "));
}

/// A `main.rs` **deeper** than that is not a target: cargo compiles the
/// `main.rs` of a directory that is directly under `tests/`, and not the
/// `main.rs` of a directory inside that one.
#[test]
fn a_main_rs_two_directories_down_is_caught() {
    let tree = Tree::new("deep-main");
    tree.crate_with_a_test("#[test]\nfn a_file() {}\n");
    let deep = tree.root.join("crates").join("thing").join("tests").join("one").join("two");
    std::fs::create_dir_all(&deep).expect("the directory is made");
    std::fs::write(deep.join("main.rs"), "#[test]\nfn another() {}\n").expect("the file is written");
    let findings = findings_of(&tree.root, "test-file");
    assert_eq!(findings.len(), 1, "the gate found: {findings:?}");
    assert!(findings[0].contains("never runs"), "the finding says: {}", findings[0]);
}

/// A test the **manifest declares** is a test cargo compiles, wherever it
/// is: the rule leaves it alone.
#[test]
fn a_test_target_the_manifest_declares_is_not_caught() {
    let tree = Tree::new("declared");
    tree.crate_with_a_test("#[test]\nfn a_file() {}\n");
    let nested = tree.root.join("crates").join("thing").join("tests").join("somewhere");
    std::fs::create_dir_all(&nested).expect("the directory is made");
    std::fs::write(nested.join("declared.rs"), "#[test]\nfn another() {}\n").expect("the file is written");
    let manifest = tree.root.join("crates").join("thing").join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).expect("the manifest is read");
    std::fs::write(
        &manifest,
        format!("{text}\n[[test]]\nname = \"declared\"\npath = \"tests/somewhere/declared.rs\"\n"),
    )
    .expect("the manifest is written");
    let findings = findings_of(&tree.root, "test-file");
    assert!(findings.is_empty(), "the gate found: {}", findings.join("\n     "));
}

/// The **cfg** form of a target is a target too, and the rule reads it.
#[test]
fn a_cfg_target_the_manifest_declares_is_not_caught() {
    let tree = Tree::new("cfg");
    tree.crate_with_a_test("#[test]\nfn a_file() {}\n");
    let nested = tree.root.join("crates").join("thing").join("tests").join("somewhere");
    std::fs::create_dir_all(&nested).expect("the directory is made");
    std::fs::write(nested.join("declared.rs"), "#[test]\nfn another() {}\n").expect("the file is written");
    let manifest = tree.root.join("crates").join("thing").join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).expect("the manifest is read");
    std::fs::write(
        &manifest,
        format!(
            "{text}\n[target.'cfg(unix)'.test]\nname = \"declared\"\npath = \
             \"tests/somewhere/declared.rs\"\n"
        ),
    )
    .expect("the manifest is written");
    let findings = findings_of(&tree.root, "test-file");
    assert!(findings.is_empty(), "the gate found: {}", findings.join("\n     "));
}

/// A manifest may write its path as `./tests/x.rs`, and that is the same
/// file as `tests/x.rs`: a rule that says otherwise reports a test.
#[test]
fn a_declared_path_written_with_a_dot_is_the_same_file() {
    let tree = Tree::new("dot");
    tree.crate_with_a_test("#[test]\nfn a_file() {}\n");
    let nested = tree.root.join("crates").join("thing").join("tests").join("somewhere");
    std::fs::create_dir_all(&nested).expect("the directory is made");
    std::fs::write(nested.join("declared.rs"), "#[test]\nfn another() {}\n").expect("the file is written");
    let manifest = tree.root.join("crates").join("thing").join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).expect("the manifest is read");
    std::fs::write(
        &manifest,
        format!("{text}\n[[test]]\nname = \"declared\"\npath = \"./tests/somewhere/declared.rs\"\n"),
    )
    .expect("the manifest is written");
    let findings = findings_of(&tree.root, "test-file");
    assert!(findings.is_empty(), "the gate found: {}", findings.join("\n     "));
}

/// The **xtask** is a crate of the tree, and its tests are read too: a
/// rule that read the tree and not the xtask would be a rule with a hole
/// in it.
#[test]
fn the_xtasks_own_tests_are_read() {
    let tree = Tree::new("xtask");
    tree.crate_with_a_test("#[test]\nfn a_file() {}\n");
    let xtask = tree.root.join("xtask");
    std::fs::create_dir_all(xtask.join("src")).expect("the directory is made");
    std::fs::create_dir_all(xtask.join("tests").join("somewhere")).expect("the directory is made");
    std::fs::write(xtask.join("Cargo.toml"), "[package]\nname = \"xtask\"\nversion = \"0.1.0\"\n")
        .expect("the manifest is written");
    std::fs::write(xtask.join("src").join("lib.rs"), format!("{DENY}\n")).expect("the source is written");
    std::fs::write(xtask.join("tests").join("somewhere").join("other.rs"), "#[test]\nfn another() {}\n")
        .expect("the file is written");
    let findings = findings_of(&tree.root, "test-file");
    assert_eq!(findings.len(), 1, "the gate found: {findings:?}");
    assert!(findings[0].contains("xtask"), "the finding names the crate: {}", findings[0]);
}

/// A test file that is a **copy** of another is caught: one test and one
/// copy, and a copy is not a test.
#[test]
fn a_test_file_that_is_a_copy_is_caught() {
    let tree = Tree::new("copy");
    tree.crate_with_a_test("#[test]\nfn a_file() {}\n");
    let copy = tree.root.join("crates").join("thing").join("tests").join("copy.rs");
    std::fs::write(&copy, "#[test]\nfn a_file() {}\n").expect("the copy is written");
    let findings = findings_of(&tree.root, "test-file");
    assert_eq!(findings.len(), 1, "the gate found: {findings:?}");
    assert!(findings[0].contains("is a copy of"), "the finding says: {}", findings[0]);
}

/// A copy in **another crate** is caught too: the rule is about the tree,
/// and not about the crate.
#[test]
fn a_test_file_that_is_a_copy_in_another_crate_is_caught() {
    let tree = Tree::new("copy-across");
    tree.crate_with_a_test("#[test]\nfn a_file() {}\n");
    let other = tree.root.join("crates").join("other");
    std::fs::create_dir_all(other.join("src")).expect("the directory is made");
    std::fs::create_dir_all(other.join("tests")).expect("the directory is made");
    std::fs::write(other.join("Cargo.toml"), "[package]\nname = \"other\"\nversion = \"0.1.0\"\n")
        .expect("the manifest is written");
    std::fs::write(other.join("src").join("lib.rs"), format!("{DENY}\n")).expect("the source is written");
    std::fs::write(other.join("tests").join("other.rs"), "#[test]\nfn a_file() {}\n")
        .expect("the copy is written");
    let findings = findings_of(&tree.root, "test-file");
    assert_eq!(findings.len(), 1, "the gate found: {findings:?}");
    assert!(findings[0].contains("is a copy of"), "the finding says: {}", findings[0]);
}

/// Two test files that are **not** the same are not a finding: the rule is
/// about copies, and not about similarity.
#[test]
fn two_test_files_that_differ_are_not_a_finding() {
    let tree = Tree::new("differ");
    tree.crate_with_a_test("#[test]\nfn a_file() {}\n");
    let other = tree.root.join("crates").join("thing").join("tests").join("other.rs");
    std::fs::write(&other, "#[test]\nfn another_file() {}\n").expect("the file is written");
    let findings = findings_of(&tree.root, "test-file");
    assert!(findings.is_empty(), "the gate found: {}", findings.join("\n     "));
}

/// An **empty** test file is not a copy of another: it carries no test, so
/// a copy of it is nothing.
#[test]
fn an_empty_test_file_is_not_a_copy() {
    let tree = Tree::new("empty");
    tree.crate_with_a_test("#[test]\nfn a_file() {}\n");
    let empty = tree.root.join("crates").join("thing").join("tests").join("empty.rs");
    std::fs::write(&empty, "").expect("the file is written");
    let findings = findings_of(&tree.root, "test-file");
    assert!(findings.is_empty(), "the gate found: {}", findings.join("\n     "));
}
