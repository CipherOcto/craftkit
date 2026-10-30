//! `craftkit-gates` — the rules of this tree, checked by a program.
//!
//! # Why this crate exists
//!
//! The outer tree has gates of its own — `cargo xtask lists` checks the
//! deny attribute in every crate root, and `cargo xtask layers` checks
//! the layering — but **`v2/` is excluded from the outer workspace**,
//! so none of them reach in here. The rules this tree lives by were
//! therefore **rules and not gates**: they were true, and nothing said
//! so.
//!
//! This crate is the gate. It has four rules:
//!
//! 1. **The deny attribute is the house one.** Every crate root carries
//!    the same six lints, in the same order, so that a deny list that
//!    varies between crates is a deny list nobody can reason about.
//! 2. **The layering holds.** A crate may depend only on crates in
//!    **strictly lower** layers. The apps are exempt: they depend on
//!    everything, by design, and every real layering mistake has been
//!    made in a library.
//! 3. **A plugin is a plugin.** A plugin may not depend on an app, and
//!    may not depend on another plugin: the seam is the SDK, the macro
//!    and the libraries, and nothing else.
//! 4. **The bounds are named in the prose.** Every public bound in the
//!    tree is named in this tree's README: a bound a reader cannot look
//!    up is a bound a reader cannot check. The outer tree has the same
//!    rule for its own tree (§71).
//!
//! # Why the gate is a function and not a script
//!
//! Because a gate that can only be run on the tree cannot be **measured**:
//! you cannot ask it whether it would catch a mistake without making
//! the mistake. [`check`] takes a root and returns what it found, so a
//! test can point it at a tree that is broken **on purpose** and say
//! that the gate catches it. That is the whole of the mutation
//! instrument, and it needs no copy of the tree and no mutation of it.
//!
//! # The layering table
//!
//! [`LAYERS`] is the table, and it is the rule: a crate that is not in
//! it is a finding, so a new crate has to be given a layer rather than
//! quietly floating.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

/// The lints, in house order.
///
/// One list, so that there is nothing to choose between.
pub const LINTS: &[&str] = &[
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::panic",
    "clippy::unimplemented",
    "clippy::todo",
    "clippy::unreachable",
];

/// The layering table: every crate of this workspace and its layer.
///
/// The names are the full package names, and the table is sorted by
/// layer and then by name, so that the table itself is reviewable as a
/// diff.
pub const LAYERS: &[(&str, u8)] = &[
    // L0 — the half that knows nothing about a document: the allocator
    // gate, the macro, the ABI, the build-time linker and the two
    // libraries that are arithmetic over plain numbers.
    ("craftkit-alloc", 0),
    ("craftkit-derive", 0),
    ("craftkit-ffi-abi", 0),
    ("craftkit-gates", 0),
    ("craftkit-link", 0),
    // The payload reader: a bounded reading of the values a document
    // carries. It knows nothing about the SDK and nothing about a
    // document: it reads a JSON value within limits.
    ("craftkit-image", 0),
    // The JPEG decoder: the family's own, and it depends on nothing.
    ("craftkit-jpeg", 0),
    // The PNG reader: RFC 1951 and RFC 2083, and it depends on nothing.
    ("craftkit-png", 0),
    ("craftkit-payload", 0),
    ("craftkit-sound-time", 0),
    // The line breaker: arithmetic over the widths a caller brings. It
    // knows nothing about a document, a font or the kit.
    ("craftkit-text-layout", 0),
    // L1 — the door: opening a shared library and calling the table in
    // it. One `unsafe` block, and it is named.
    ("craftkit-ffi", 1),
    // L2 — the whole surface a plugin needs: the registry, the session,
    // the document and the format.
    ("craftkit-sdk", 2),
    // L3 — the things that sit above the SDK and are not plugins: the
    // MCP server, the control channel, and the vector geometry library,
    // which speaks the SDK's own types.
    ("craftkit-control", 3),
    ("craftkit-mcp", 3),
    // The numeric library is here and not at L0 because a **length** can
    // be a command's parameter, and that means implementing the kit's
    // own `ParamKind` — which is speaking the SDK's types. §56.
    ("craftkit-numeric", 3),
    // The rasteriser: the canvas the picture is drawn on. It is here and
    // not at L0 because it speaks the SDK's own types — the canvas
    // trait, the document's nodes and the camera.
    ("craftkit-raster", 3),
    ("craftkit-vector-geom", 3),
    // L4 — the batteries: the plugins, each one a document model of its
    // own. A plugin may not depend on another plugin, and that is a
    // rule and not a hope.
    ("craftkit-plugin-art", 4),
    ("craftkit-plugin-cad", 4),
    ("craftkit-plugin-film", 4),
    ("craftkit-plugin-fx", 4),
    ("craftkit-plugin-grid", 4),
    ("craftkit-plugin-hello", 4),
    ("craftkit-plugin-image", 4),
    ("craftkit-plugin-layout", 4),
    ("craftkit-plugin-numeric", 4),
    ("craftkit-plugin-pdf", 4),
    ("craftkit-plugin-prose", 4),
    ("craftkit-plugin-shapes", 4),
    ("craftkit-plugin-text", 4),
    ("craftkit-plugin-shared", 4),
    ("craftkit-plugin-sound", 4),
    ("craftkit-plugin-vector", 4),
    // L5 — the applications. They depend on everything, and the layering
    // rule does not apply to them.
    ("craftkit-bridge", 5),
    ("craftkit-hello-server", 5),
    ("craftkit-mcp-server", 5),
    ("craftkit-studio", 5),
];

/// The prefix a plugin's name carries.
const PLUGIN_PREFIX: &str = "craftkit-plugin-";

/// What the gate found.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    /// Which rule was broken.
    pub rule: &'static str,
    /// Which crate it was broken in.
    pub crate_: String,
    /// What is wrong, in one line.
    pub message: String,
}

/// The deny attribute, in the form the formatter leaves it.
pub const DENY: &str = concat!(
    "#![deny(\n",
    "    clippy::unwrap_used,\n",
    "    clippy::expect_used,\n",
    "    clippy::panic,\n",
    "    clippy::unimplemented,\n",
    "    clippy::todo,\n",
    "    clippy::unreachable\n",
    ")]"
);

/// The deny attribute as it is compared: no whitespace, lower case, and
/// no `clippy::`.
///
/// The formatting of the attribute is the formatter's business and not
/// the gate's: one line and six lines are the same six lints.
pub fn normalise(body: &str) -> Option<String> {
    let start = body.find("#![deny(")?;
    let rest = &body[start + "#![deny(".len()..];
    let end = rest.find(")]")?;
    Some(strip(&rest[..end]))
}

/// The house list in the same form [`normalise`] gives.
pub fn canonical() -> String {
    let inner = DENY.strip_prefix("#![deny(").and_then(|rest| rest.strip_suffix(")]")).unwrap_or(DENY);
    strip(inner)
}

/// Strip what the comparison does not care about.
fn strip(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase().replace("clippy::", "")
}

/// A crate of the tree.
#[derive(Debug, Clone)]
struct Crate {
    /// Its package name.
    name: String,
    /// Where it lives.
    dir: PathBuf,
    /// Whether it is an application.
    app: bool,
    /// The crates it depends on, and the ones it only needs for its
    /// tests.
    ///
    /// The two are apart because a **dev** dependency is a testing
    /// arrangement: nothing that ships links it. The door's loader tests
    /// need a shared library to open, and that library is a plugin — and
    /// that is the right way round, because the plugin is not part of
    /// anything that ships.
    depends: Vec<String>,
    dev_depends: Vec<String>,
}

impl Crate {
    /// Its layer, if the table has it.
    fn layer(&self) -> Option<u8> {
        LAYERS.iter().find(|(name, _)| *name == self.name).map(|(_, layer)| *layer)
    }

    /// Its root: the library, or the binary when there is no library.
    fn root(&self) -> Option<PathBuf> {
        let lib = self.dir.join("src").join("lib.rs");
        if lib.exists() {
            return Some(lib);
        }
        let main = self.dir.join("src").join("main.rs");
        if main.exists() {
            return Some(main);
        }
        None
    }
}

/// Check the tree at `root`, and say what is wrong with it.
pub fn check(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    let crates = crates(root);

    for crate_ in &crates {
        // Rule 1: the deny attribute is the house one.
        match crate_.root() {
            None => findings.push(Finding {
                rule: "deny",
                crate_: crate_.name.clone(),
                message: "the crate has no root: there is no `src/lib.rs` and no `src/main.rs`".to_string(),
            }),
            Some(path) => {
                let code = std::fs::read_to_string(&path).unwrap_or_default();
                match normalise(&code) {
                    None => findings.push(Finding {
                        rule: "deny",
                        crate_: crate_.name.clone(),
                        message: format!("`{}` has no deny attribute", path.display()),
                    }),
                    Some(found) if found == canonical() => {}
                    Some(found) => findings.push(Finding {
                        rule: "deny",
                        crate_: crate_.name.clone(),
                        message: format!("`{}` does not carry the house lints: {found}", path.display()),
                    }),
                }
            }
        }

        // The layer of a crate that is not in the table.
        if crate_.layer().is_none() {
            findings.push(Finding {
                rule: "layering",
                crate_: crate_.name.clone(),
                message: "the crate is not in the layering table".to_string(),
            });
        }
    }

    for crate_ in &crates {
        let Some(layer) = crate_.layer() else { continue };
        // A **dev** dependency is a testing arrangement and does not
        // ship, so the layering and the plugin rule do not apply to it.
        for dependency in crate_.depends.iter().chain(crate_.dev_depends.iter()) {
            let is_dev = crate_.dev_depends.contains(dependency);
            if is_dev {
                continue;
            }
            let Some(dependency_layer) = LAYERS.iter().find(|(name, _)| name == dependency).map(|(_, l)| *l)
            else {
                findings.push(Finding {
                    rule: "layering",
                    crate_: crate_.name.clone(),
                    message: format!("the dependency `{dependency}` is not in the layering table"),
                });
                continue;
            };
            // Rule 2: the layering holds. The apps are exempt.
            if !crate_.app && dependency_layer >= layer {
                findings.push(Finding {
                    rule: "layering",
                    crate_: crate_.name.clone(),
                    message: format!(
                        "depends on `{dependency}`, which is L{dependency_layer} and not lower than L{layer}"
                    ),
                });
            }
            // Rule 3: a plugin is a plugin.
            if crate_.name.starts_with(PLUGIN_PREFIX) {
                if dependency.starts_with(PLUGIN_PREFIX) {
                    findings.push(Finding {
                        rule: "plugin",
                        crate_: crate_.name.clone(),
                        message: format!("depends on the plugin `{dependency}`"),
                    });
                }
                if is_app(dependency) {
                    findings.push(Finding {
                        rule: "plugin",
                        crate_: crate_.name.clone(),
                        message: format!("depends on the application `{dependency}`"),
                    });
                }
            }
        }
    }

    // Rule 4: the bounds are named in the prose.
    bounds_named_in_the_prose(root, &mut findings);

    findings
}

/// Rule 4: **the bounds are named in the prose**.
///
/// A bound that no document names is a bound a reader cannot check: the
/// number is in the code and the reason for it is nowhere. The outer
/// tree has this rule for its own tree; this is the same rule for this
/// one, and the prose it checks is **this** tree's README.
///
/// Only the **public** bounds are checked: a private cap is the
/// program's own, and the README names those too — but a private cap is
/// not a promise anybody else may rely on, and the rule is about the
/// promises.
fn bounds_named_in_the_prose(root: &Path, findings: &mut Vec<Finding>) {
    // A tree with no README has no prose, and a bound with no prose is
    // a bound no reader can look up: the rule is about the bounds, and
    // it fires for each of them.
    let prose = std::fs::read_to_string(root.join("README.md")).unwrap_or_default();
    let mut names: Vec<(String, String)> = Vec::new();
    for crate_ in crates(root) {
        for file in rust_files(&crate_.dir) {
            let Ok(code) = std::fs::read_to_string(&file) else { continue };
            for name in public_bounds(&code) {
                let found = name.clone();
                if !names.iter().any(|(other, _)| *other == found) {
                    names.push((found, file.display().to_string()));
                }
            }
        }
    }
    for (name, file) in names {
        if !names_the_prose(&prose, &name) {
            findings.push(Finding {
                rule: "prose",
                crate_: "the tree".to_string(),
                message: format!(
                    "{file} has the public bound `{name}` and no document in the tree names it; a bound a reader cannot look up is a bound a reader cannot check"
                ),
            });
        }
    }
}

/// The public bounds a file declares.
///
/// A bound is a `MAX_` constant whose name says what it bounds: bytes,
/// memory or steps. The names are the ones the outer tree's rule uses,
/// so that a reader of either tree knows what is being looked for.
fn public_bounds(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in code.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("pub const ") else { continue };
        let Some((name, _)) = rest.split_once(':') else { continue };
        let name = name.trim();
        if !name.starts_with("MAX_") {
            continue;
        }
        if !(name.ends_with("BYTES") || name.ends_with("MEMORY") || name.ends_with("STEPS")) {
            continue;
        }
        out.push(name.to_string());
    }
    out
}

/// Whether the prose names that bound.
///
/// A name is named when it is in the text as a **word**: a bound that
/// is a prefix of another bound's name is not named.
fn names_the_prose(prose: &str, name: &str) -> bool {
    prose.split(|character: char| !character.is_alphanumeric() && character != '_').any(|word| word == name)
}

/// The Rust files of a crate.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir.join("src")) else { return out };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// Whether a package name is one of the applications.
fn is_app(name: &str) -> bool {
    LAYERS.iter().find(|(crate_name, _)| *crate_name == name).map(|(_, layer)| *layer == 5).unwrap_or(false)
}

/// The crates of the tree at `root`.
fn crates(root: &Path) -> Vec<Crate> {
    let mut out = Vec::new();
    for (folder, app) in [("crates", false), ("apps", true)] {
        let Ok(entries) = std::fs::read_dir(root.join(folder)) else { continue };
        let mut folders: Vec<PathBuf> =
            entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
        folders.sort();
        for dir in folders {
            let manifest = dir.join("Cargo.toml");
            let Ok(text) = std::fs::read_to_string(&manifest) else { continue };
            let Some(name) = package_name(&text) else { continue };
            out.push(Crate {
                name,
                dir,
                app,
                depends: dependencies(&text, &["dependencies", "build-dependencies"]),
                dev_depends: dependencies(&text, &["dev-dependencies"]),
            });
        }
    }
    out
}

/// The package's name, from its manifest.
fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some((name, value)) = line.split_once('=')
            && name.trim() == "name"
        {
            return Some(value.trim().trim_matches('"').to_string());
        }
    }
    None
}

/// The crates a manifest depends on, in the sections that are named.
fn dependencies(manifest: &str, sections: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let mut section = "";
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = &line[1..line.len() - 1];
            continue;
        }
        if !sections.contains(&section) {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            let name = name.trim();
            if name.starts_with("craftkit-") && !out.contains(&name.to_string()) {
                out.push(name.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_deny_attribute_is_matched_whatever_the_formatting() {
        let one_line = "#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]";
        assert_eq!(normalise(one_line).as_deref(), Some(canonical().as_str()));
        assert_eq!(normalise(DENY).as_deref(), Some(canonical().as_str()));
    }

    #[test]
    fn a_deny_of_a_different_set_of_lints_is_not_the_house_one() {
        let shorter = "#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable)]";
        assert_ne!(normalise(shorter).as_deref(), Some(canonical().as_str()));
    }

    #[test]
    fn a_deny_in_a_different_order_is_not_the_house_one() {
        let swapped = "#![deny(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]";
        assert_ne!(normalise(swapped).as_deref(), Some(canonical().as_str()));
    }

    #[test]
    fn a_crate_with_no_deny_attribute_is_not_the_house_one() {
        assert_eq!(normalise("fn main() {}"), None);
    }

    #[test]
    fn the_dependencies_of_a_manifest_are_read() {
        let manifest = "[package]\nname = \"thing\"\n\n[dependencies]\ncraftkit-sdk = { path = \"../sdk\" }\nserde_json = \"1\"\n\n[dev-dependencies]\ncraftkit-alloc = { path = \"../alloc\" }\n";
        let deps = dependencies(manifest, &["dependencies", "dev-dependencies"]);
        assert!(deps.contains(&"craftkit-sdk".to_string()));
        assert!(deps.contains(&"craftkit-alloc".to_string()));
        assert!(!deps.contains(&"serde_json".to_string()));
    }

    #[test]
    fn the_package_name_is_read() {
        let manifest = "[package]\nname = \"craftkit-thing\"\nversion = \"0.1.0\"\n\n[dependencies]\n";
        assert_eq!(package_name(manifest).as_deref(), Some("craftkit-thing"));
    }
}
