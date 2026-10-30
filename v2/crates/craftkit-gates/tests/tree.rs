//! The gate, measured: the tree is clean, and the gate catches the
//! trees that are broken on purpose.
//!
//! The second half is the point. A gate that is only ever run on a
//! good tree has never been shown to **bite**, and a gate that cannot
//! bite is a comment.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use craftkit_gates::{DENY, Finding, check};

/// The root of this workspace.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("the root is there")
}

/// The tree is clean: the gate finds nothing.
#[test]
fn the_tree_is_clean() {
    let findings = check(&root());
    let report: Vec<String> =
        findings.iter().map(|finding| format!("{}: {}", finding.rule, finding.message)).collect();
    assert!(findings.is_empty(), "the gate found: {}", report.join("\n     "));
}

/// A tree built in a temporary directory, so that it can be broken on
/// purpose.
struct Tree {
    root: PathBuf,
}

impl Tree {
    /// A new tree, with a name of its own so that two tests do not
    /// fight over one directory.
    fn new(name: &str) -> Tree {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("craftkit-gates-{}-{name}-{unique}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the directory is made");
        Tree { root }
    }

    /// A crate in the tree: its manifest and its root.
    ///
    /// The deny attribute is the house one unless `deny` says otherwise,
    /// and the dependencies are the ones that are named.
    fn crate_(&self, folder: &str, name: &str, depends: &[&str], deny: Option<&str>) {
        let dir = self.root.join(folder).join(name);
        std::fs::create_dir_all(dir.join("src")).expect("the directory is made");
        let mut manifest = format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n\n[dependencies]\n");
        for dependency in depends {
            manifest.push_str(&format!("{dependency} = {{ path = \"../{dependency}\" }}\n"));
        }
        std::fs::write(dir.join("Cargo.toml"), manifest).expect("the manifest is written");
        let attribute = deny.unwrap_or(DENY);
        std::fs::write(dir.join("src").join("lib.rs"), format!("{attribute}\n\npub fn nothing() {{}}\n"))
            .expect("the root is written");
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// The findings of one rule.
fn findings_of<'a>(findings: &'a [Finding], rule: &str) -> Vec<&'a Finding> {
    findings.iter().filter(|finding| finding.rule == rule).collect()
}

/// A deny attribute that is **not** the house one is caught.
#[test]
fn a_deny_attribute_that_is_not_the_house_one_is_caught() {
    let tree = Tree::new("deny");
    tree.crate_("crates", "craftkit-sdk", &[], Some("#![deny(clippy::unwrap_used)]"));
    let findings = check(&tree.root);
    let deny = findings_of(&findings, "deny");
    assert_eq!(deny.len(), 1, "the gate caught it: {findings:?}");
    assert!(deny[0].message.contains("house lints"), "{}", deny[0].message);
}

/// A crate with **no** deny attribute is caught.
#[test]
fn a_crate_with_no_deny_attribute_is_caught() {
    let tree = Tree::new("nodeny");
    tree.crate_("crates", "craftkit-sdk", &[], Some("pub fn nothing() {}"));
    let findings = check(&tree.root);
    assert_eq!(findings_of(&findings, "deny").len(), 1, "the gate caught it: {findings:?}");
}

/// A crate that depends on a **higher** layer is caught.
#[test]
fn a_crate_that_depends_on_a_higher_layer_is_caught() {
    let tree = Tree::new("higher");
    // The SDK is L2 and the MCP server is L3: the SDK may not depend on
    // the MCP server.
    tree.crate_("crates", "craftkit-sdk", &["craftkit-mcp"], None);
    let findings = check(&tree.root);
    let layering = findings_of(&findings, "layering");
    assert_eq!(layering.len(), 1, "the gate caught it: {findings:?}");
    assert!(layering[0].message.contains("craftkit-mcp"), "{}", layering[0].message);
}

/// A crate that depends on the **same** layer is caught.
#[test]
fn a_crate_that_depends_on_the_same_layer_is_caught() {
    let tree = Tree::new("same");
    // The MCP server is L3 and the vector geometry library is L3.
    tree.crate_("crates", "craftkit-mcp", &["craftkit-vector-geom"], None);
    let findings = check(&tree.root);
    assert_eq!(findings_of(&findings, "layering").len(), 1, "the gate caught it: {findings:?}");
}

/// A plugin that depends on **another plugin** is caught.
#[test]
fn a_plugin_that_depends_on_another_plugin_is_caught() {
    let tree = Tree::new("plugin");
    tree.crate_("crates", "craftkit-plugin-art", &["craftkit-plugin-cad"], None);
    let findings = check(&tree.root);
    let plugin = findings_of(&findings, "plugin");
    assert_eq!(plugin.len(), 1, "the gate caught it: {findings:?}");
    assert!(plugin[0].message.contains("craftkit-plugin-cad"), "{}", plugin[0].message);
}

/// A plugin that depends on an **application** is caught.
#[test]
fn a_plugin_that_depends_on_an_application_is_caught() {
    let tree = Tree::new("app");
    tree.crate_("crates", "craftkit-plugin-art", &["craftkit-studio"], None);
    let findings = check(&tree.root);
    let plugin = findings_of(&findings, "plugin");
    assert_eq!(plugin.len(), 1, "the gate caught it: {findings:?}");
    assert!(plugin[0].message.contains("application"), "{}", plugin[0].message);
}

/// A crate that is **not in the layering table** is caught: a new crate
/// has to be given a layer rather than quietly floating.
#[test]
fn a_crate_that_is_not_in_the_layering_table_is_caught() {
    let tree = Tree::new("floating");
    tree.crate_("crates", "craftkit-new-thing", &[], None);
    let findings = check(&tree.root);
    let layering = findings_of(&findings, "layering");
    assert_eq!(layering.len(), 1, "the gate caught it: {findings:?}");
    assert!(layering[0].message.contains("not in the layering table"), "{}", layering[0].message);
}

/// A plugin that depends on the SDK and a library is **not** caught:
/// that is what a plugin is allowed to do.
#[test]
fn a_plugin_that_depends_on_the_sdk_and_a_library_is_fine() {
    let tree = Tree::new("fine");
    tree.crate_("crates", "craftkit-plugin-numeric", &["craftkit-sdk", "craftkit-numeric"], None);
    let findings = check(&tree.root);
    assert!(findings.is_empty(), "the gate found: {findings:?}");
}
