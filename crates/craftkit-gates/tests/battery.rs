//! The rule that **a battery is a table**, and that the gate bites.
//!
//! A gate that is only ever run on a good tree has never been shown to
//! **bite**, and a gate that cannot bite is a comment. So this file breaks
//! a tree on purpose — a battery that declares a command the other way —
//! and says that the gate finds it.
//!
//! # Why the rule exists
//!
//! Every battery of the tree declares its commands as rows of a table, and
//! the compiler writes the struct, the reader and the registration for
//! every row (§860–§869). A battery that declares one command the other
//! way — a struct with `#[derive(Command)]` — is a battery that is half
//! one spelling, and the half that is not a table is the half a reader
//! cannot see at a glance.
//!
//! The rule is checked against the **code**: a battery that *talks* about
//! the derive — in a doc comment, in a string — is not a battery that uses
//! it, and the rule says so.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use craftkit_gates::{BATTERIES, DENY, check};

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
            craftkit_scratch::dir(&format!("craftkit-gates-battery-{}-{name}-{unique}", std::process::id()))
                .expect("the directory is made");
        Tree { root }
    }

    /// A **battery** in the tree, with the code it declares its commands
    /// with.
    fn battery(&self, body: &str) -> &Tree {
        let dir = self.root.join("crates").join("craftkit-plugin-cad");
        std::fs::create_dir_all(dir.join("src")).expect("the directory is made");
        let manifest = "[package]\nname = \"craftkit-plugin-cad\"\nversion = \"0.1.0\"\n";
        std::fs::write(dir.join("Cargo.toml"), manifest).expect("the manifest is written");
        let source = format!("{DENY}\n\n{body}\n");
        std::fs::write(dir.join("src").join("lib.rs"), source).expect("the source is written");
        self
    }
}

/// The findings of a rule, in a tree.
fn findings_of(root: &Path, rule: &str) -> Vec<String> {
    check(root).into_iter().filter(|finding| finding.rule == rule).map(|finding| finding.message).collect()
}

/// The tree of the workspace: no battery declares a command the other way.
#[test]
fn no_battery_of_the_tree_uses_the_other_spelling() {
    let findings = findings_of(&root(), "battery");
    assert!(findings.is_empty(), "the gate found: {}", findings.join("\n     "));
}

/// The nine batteries are the ones the rule names.
#[test]
fn the_rule_names_the_nine_batteries() {
    assert_eq!(BATTERIES.len(), 9, "the tree counts nine batteries");
    for battery in BATTERIES {
        assert!(battery.starts_with("craftkit-plugin-"), "`{battery}` is a plugin's name");
    }
}

/// A battery that declares a command with the derive is found.
#[test]
fn a_battery_that_uses_the_derive_is_found() {
    let tree = Tree::new("derive");
    tree.battery(
        "/// The command.\n\
         #[derive(Command)]\n\
         #[command(id = \"cad.group.add\", area = \"cad\")]\n\
         pub struct AddGroup {\n\
         }\n",
    );
    let findings = findings_of(&tree.root, "battery");
    assert_eq!(findings.len(), 1, "the gate found: {}", findings.join("\n     "));
    assert!(findings[0].contains("derive(Command)"), "the finding names the derive: {}", findings[0]);
}

/// A battery that declares its commands as rows is **not** found.
#[test]
fn a_battery_of_tables_is_not_found() {
    let tree = Tree::new("table");
    tree.battery(
        "commands! {\n\
         \x20   /// The command.\n\
         \x20   cad.group.add {\n\
         \x20       area: \"cad\",\n\
         \x20       params: {\n\
         \x20       },\n\
         \x20       run: {\n\
         \x20       }\n\
         \x20   }\n\
         }\n",
    );
    let findings = findings_of(&tree.root, "battery");
    assert!(findings.is_empty(), "the gate found: {}", findings.join("\n     "));
}

/// A battery that **talks** about the derive is not found: a name in a
/// comment is not a declaration, and a name in a string is not a
/// declaration either.
#[test]
fn a_battery_that_talks_about_the_derive_is_not_found() {
    let tree = Tree::new("talk");
    tree.battery(
        "/// The old spelling was `#[derive(Command)]`, and this battery does not use it.\n\
         pub const NOTE: &str = \"#[derive(Command)]\";\n",
    );
    let findings = findings_of(&tree.root, "battery");
    assert!(findings.is_empty(), "the gate found: {}", findings.join("\n     "));
}
