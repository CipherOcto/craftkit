//! The layering, checked when the tool is **built**.
//!
//! # Why this is here
//!
//! The layering is a fact about the tree's manifests, and a fact that is
//! checked when the gate *runs* is a fact that is checked **late**: the gate
//! is the last thing an arc runs, so a layering mistake that only shows up
//! there is a mistake that was written, compiled, and believed to be right.
//! A build script sees the same facts at build time, and it says the
//! crate's name.
//!
//! # Why the rules are written twice
//!
//! The xtask's manifest says "**Nothing of the tree's own**", and that is
//! the whole reason this file is not a call into `xtask::layers`: a checker
//! that cannot be built when the tree does not compile is a checker that is
//! silent exactly when it is needed. So this script reads the tree **as
//! text** — the manifests and the layering table — and it does not run
//! `cargo metadata`, which would make the build slow and would make the
//! script depend on a toolchain.
//!
//! The duplication is deliberate, and the two copies are kept honest by the
//! gate: `xtask/src/layers.rs` holds the same rules and the same table, and
//! `cargo xtask layers` compares its table with the authority's. This file
//! has no table of its own — it reads the authority, `crates/craftkit-gates/
//! src/lib.rs`, which is the tree's own and the only one.
//!
//! # What it checks, and what it does not
//!
//! The three rules that are about the manifest graph:
//!
//! - a crate may depend only on crates in **strictly lower** layers;
//! - the **applications** are exempt: they depend on everything, by design;
//! - a **plugin** may not depend on another plugin, nor on an application.
//!
//! A **dev** dependency is a testing arrangement and does not ship, so it
//! does not count — the door's loader tests need a shared library to open,
//! and that library is a plugin.
//!
//! A crate that the table does not know is **not** checked here: it has no
//! layer, so there is nothing to compare. The tree's own gate says so, and
//! it says it where the crate's name is.

// A build script is a crate root too, and it holds the house rule like every
// other one: a script that panics takes the build down with a stack trace
// instead of a sentence.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The file that holds the tree's own layering table.
///
/// **The authority.** Every other copy of the table is a copy of this one.
const AUTHORITY: &str = "crates/craftkit-gates/src/lib.rs";

/// The layer the applications are at.
///
/// The applications are the only crates at this layer, so the layer number
/// is also how the rules say "this is an application".
const APP_LAYER: u8 = 5;

/// The prefix a plugin's name carries.
///
/// A plugin is a battery: a document model of its own, reached through the
/// SDK.
const PLUGIN_PREFIX: &str = "craftkit-plugin-";

fn main() {
    let manifest_dir = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(directory) => directory,
        Err(error) => {
            eprintln!("the layering could not be checked: there is no manifest directory ({error})");
            std::process::exit(1);
        }
    };
    let root = Path::new(&manifest_dir).parent().unwrap_or(Path::new("."));

    // The build script reads the tree, so it has to be told when the tree
    // changes. Every manifest is named, and so is the table.
    println!("cargo:rerun-if-changed={}", root.join(AUTHORITY).display());
    for manifest in manifests(root) {
        println!("cargo:rerun-if-changed={}", manifest.1.display());
    }

    if let Err(problem) = check(root) {
        // A build script's failure is a **message**, not a panic: the reader
        // is a person, and the person is told what is wrong.
        eprintln!("{problem}");
        std::process::exit(1);
    }
}

/// Check every rule against the tree at `root`.
///
/// The error is the report: every mistake, one per line, and the line says
/// which crate and which dependency.
fn check(root: &Path) -> Result<(), String> {
    let table = layers(root)?;
    let mut problems: Vec<String> = Vec::new();

    for (name, manifest) in manifests(root) {
        let Some(layer) = table.get(&name) else {
            // The table does not know this crate, so it has no layer and
            // there is nothing to compare. The tree's own gate says so.
            continue;
        };
        // An application depends on everything, by design.
        if *layer == APP_LAYER {
            continue;
        }
        let text = read(&manifest)?;
        for dependency in dependencies(&text) {
            // A crate depending on itself is a feature trick, not a cycle.
            if dependency == name {
                continue;
            }
            let Some(to) = table.get(&dependency) else {
                // Not a crate of this tree: an outside crate, and the
                // layering has nothing to say about it.
                continue;
            };
            // A plugin is a plugin: a plugin reaching for another plugin, or
            // for an application, is reported as that, and not again as an
            // upward dependency. The specific rule says more than the
            // general one.
            if name.starts_with(PLUGIN_PREFIX) {
                if dependency.starts_with(PLUGIN_PREFIX) {
                    problems.push(format!(
                        "{name} depends on {dependency}, which is another plugin.\n\
                         \x20     A plugin reaches the rest of the tree through the SDK; a plugin \
                         that reaches for a plugin has reached around the seam."
                    ));
                    continue;
                }
                if *to == APP_LAYER {
                    problems.push(format!(
                        "{name} depends on {dependency}, which is an application.\n\
                         \x20     A plugin is a library: it may not depend on a program."
                    ));
                    continue;
                }
            }
            // Strictly lower. Not "lower or equal": an equal-layer
            // dependency is how two crates quietly become one.
            if *to >= *layer {
                problems.push(format!(
                    "{name} (L{layer}) depends on {dependency} (L{to}): a crate may only depend \
                     on strictly lower layers."
                ));
            }
        }
    }

    if problems.is_empty() {
        return Ok(());
    }
    problems.sort();
    problems.dedup();
    Err(format!(
        "the layering of the tree is wrong, and this is the build that found it:\n{}",
        problems.join("\n")
    ))
}

/// The tree's own layering table, read out of its source.
///
/// The table is the authority's, and it is read as text because the xtask
/// depends on nothing of the tree's own. A table that is not written as a
/// table is a **failure**, and not an empty table: this says which line it
/// could not read.
fn layers(root: &Path) -> Result<BTreeMap<String, u8>, String> {
    let path = root.join(AUTHORITY);
    let text = read(&path)?;
    let header = "pub const LAYERS";
    let Some(start) = text.find(header) else {
        return Err(format!("{AUTHORITY} has no `{header}`: the layering table could not be read"));
    };
    let after = &text[start..];
    let Some(open) = after.find("= &[") else {
        return Err(format!("{AUTHORITY} does not declare `{header}` as a table of `(&str, u8)` rows"));
    };
    let block = &after[open + "= &[".len()..];

    let mut table = BTreeMap::new();
    for (number, line) in block.lines().enumerate() {
        let line = line.trim();
        if line.starts_with("];") {
            break;
        }
        // A comment is not a row.
        let line = line.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some(open) = line.find('(') else { continue };
        let Some(close) = line.find(')') else {
            return Err(format!("{AUTHORITY}: the row on line {} has no closing bracket", number + 1));
        };
        let mut cells = line[open + 1..close].split(',');
        let name = cells.next().unwrap_or("").trim().trim_matches('"').to_string();
        let level = cells.next().unwrap_or("").trim();
        if name.is_empty() || level.is_empty() {
            return Err(format!("{AUTHORITY}: the row on line {} is not `(\"name\", layer)`", number + 1));
        }
        let Ok(level) = level.parse::<u8>() else {
            return Err(format!(
                "{AUTHORITY}: the row on line {} has the layer `{level}`, which is not a number",
                number + 1
            ));
        };
        table.insert(name, level);
    }
    if table.is_empty() {
        return Err(format!("{AUTHORITY} declares an empty layering table"));
    }
    Ok(table)
}

/// The manifests of the tree's crates, with their names.
///
/// The crates are the folders under `crates/` and `apps/`, and the name is
/// the manifest's own.
fn manifests(root: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    for folder in ["crates", "apps"] {
        let Ok(entries) = std::fs::read_dir(root.join(folder)) else { continue };
        let mut folders: Vec<PathBuf> =
            entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
        folders.sort();
        for directory in folders {
            let manifest = directory.join("Cargo.toml");
            let Ok(text) = std::fs::read_to_string(&manifest) else { continue };
            let Some(name) = package_name(&text) else { continue };
            out.push((name, manifest));
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

/// The crates a manifest depends on, in the sections that ship.
///
/// The **dev** dependencies are not here: a testing arrangement does not
/// ship, and the tree's own gate does not apply the layering rules to one.
fn dependencies(manifest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut section = "";
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            section = trimmed.trim_start_matches('[').trim_end_matches(']');
            continue;
        }
        // A dependency is a **top-level** key of the section: an entry of an
        // inline table is indented, and a line of a multi-line table is not
        // a key at all.
        if line.starts_with(char::is_whitespace) || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let is_dependency = section == "dependencies"
            || section == "build-dependencies"
            || (section.ends_with(".dependencies") && !section.ends_with("dev-dependencies"));
        if !is_dependency {
            continue;
        }
        let Some((name, _)) = line.split_once('=') else { continue };
        let name = name.trim().trim_matches('"').to_string();
        if name.is_empty() {
            continue;
        }
        out.push(name);
    }
    out
}

/// Read a file, and say which file when it cannot be read.
fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|error| format!("could not read {}: {error}", path.display()))
}
