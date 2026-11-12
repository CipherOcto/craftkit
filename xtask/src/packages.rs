//! The workspace's packages, and the graph between them.
//!
//! **One place that knows what the workspace is.** The gate needs the list of
//! packages to format, to lint and to test them; the verify command needs the
//! same list *and* the dependencies between them, because a change to the SDK
//! is a change to everything that depends on it.
//!
//! Two questions about the tree, one answer each, read from **cargo's own
//! metadata** rather than from the manifests as text: the tree is what cargo
//! says it is, and a list kept by hand is a list that drifts.
//!
//! # Why the graph matters
//!
//! A change to `craftkit-sdk` is a change to every battery, and a change to
//! `craftkit-plugin-grid` is a change to the studio that links it. Checking
//! the wrong set is the two mistakes that cost the most time: checking
//! everything (slow, and it hides what actually broke) and checking only the
//! file that was edited (fast, and it misses the crate that used it).
//!
//! The **blast radius** of a change is the set of packages that depend on it,
//! directly or through others. [`blast_radius`] computes it, and it is the
//! one definition of "what this change can break" that the tree has.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::command;

/// One package of the workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// The package name, as cargo calls it.
    pub name: String,
    /// Cargo's own id for the package, which is how a build message says which
    /// package it is about.
    pub id: String,
    /// The directory the package lives in. Cargo runs a package's test
    /// binaries with that directory as the working directory, and a test that
    /// opens a file by a relative path depends on it.
    pub dir: PathBuf,
    /// The same directory, **relative to the workspace root**.
    ///
    /// Git says what changed in paths relative to the root, and cargo says
    /// where a package is in absolute ones. Comparing the two needs one
    /// spelling, and this is it: a package owns a file when the file's
    /// relative path starts with the package's relative directory.
    pub rel_dir: PathBuf,
    /// Whether the package has a library target, and therefore doc tests.
    pub has_lib: bool,
    /// Whether the package's library is also a **shared library**.
    ///
    /// A shared library is a file on disk, and a test that opens it with
    /// `dlopen` needs the file to exist before the test runs. Cargo writes
    /// that file only when the package is **built**, not when it is used as a
    /// dependency — so a package that is only ever a dev-dependency of another
    /// package never gets its `.so` written, and the test that opens it fails
    /// with a file that is not there.
    pub is_cdylib: bool,
    /// The workspace packages this one depends on, by name.
    pub deps: Vec<String>,
}

impl Package {
    /// Whether this package owns a path, given **relative to the root**.
    ///
    /// Ownership is by **directory**: a file under `crates/craftkit-plugin-grid`
    /// belongs to that package, and a file at the root of the workspace
    /// belongs to no package at all.
    pub fn owns(&self, relative: &Path) -> bool {
        relative.starts_with(&self.rel_dir)
    }
}

/// The packages of the workspace, in the order cargo lists them.
///
/// Read from cargo's own metadata, so a new crate is tested the moment it is
/// created and not the moment somebody remembers.
pub fn workspace_packages(root: &Path) -> Result<Vec<Package>, String> {
    let out = command("cargo", &["metadata", "--no-deps", "--format-version", "1", "--offline"], root)
        .or_else(|_| command("cargo", &["metadata", "--no-deps", "--format-version", "1"], root))?;
    if !out.ok() {
        return Err(format!("cargo metadata failed\n\n{}", out.all()));
    }
    let value: Value =
        serde_json::from_str(&out.stdout).map_err(|e| format!("cargo metadata is not JSON: {e}"))?;
    let packages = value
        .get("packages")
        .and_then(|p| p.as_array())
        .ok_or_else(|| "cargo metadata has no packages".to_string())?;

    let mut found: Vec<Package> = Vec::new();
    for entry in packages {
        let id = entry
            .get("id")
            .and_then(|i| i.as_str())
            .ok_or_else(|| "a package in cargo metadata has no id".to_string())?
            .to_string();
        let name = entry
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| "a package in cargo metadata has no name".to_string())?
            .to_string();
        let manifest = entry
            .get("manifest_path")
            .and_then(|m| m.as_str())
            .ok_or_else(|| format!("{name} has no manifest path in cargo metadata"))?;
        let dir = Path::new(manifest)
            .parent()
            .ok_or_else(|| format!("{name} has no directory in cargo metadata"))?
            .to_path_buf();
        let targets = entry.get("targets").and_then(|t| t.as_array());
        let has_lib = targets
            .map(|targets| {
                targets.iter().any(|t| {
                    t.get("kind")
                        .and_then(|k| k.as_array())
                        .map(|kinds| kinds.iter().any(|k| k.as_str() == Some("lib")))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        let is_cdylib = targets
            .map(|targets| {
                targets.iter().any(|t| {
                    t.get("crate_types")
                        .and_then(|c| c.as_array())
                        .map(|kinds| kinds.iter().any(|k| k.as_str() == Some("cdylib")))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        let mut deps: Vec<String> = Vec::new();
        if let Some(list) = entry.get("dependencies").and_then(|d| d.as_array()) {
            for dependency in list {
                // A **dev** dependency is a testing arrangement and does not
                // ship, so it is not part of the blast radius of a change.
                if dependency.get("kind").and_then(|k| k.as_str()) == Some("dev") {
                    continue;
                }
                let Some(dep_name) = dependency.get("name").and_then(|n| n.as_str()) else {
                    continue;
                };
                if !deps.contains(&dep_name.to_string()) {
                    deps.push(dep_name.to_string());
                }
            }
        }
        let rel_dir = dir.strip_prefix(root).map(Path::to_path_buf).unwrap_or_else(|_| dir.clone());
        found.push(Package { name, id, dir, rel_dir, has_lib, is_cdylib, deps });
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));

    // Only the dependencies that are **inside** the workspace are edges of the
    // graph. A dependency on a crate from crates.io is not a package that
    // changes when this one does.
    let names: Vec<String> = found.iter().map(|p| p.name.clone()).collect();
    for package in &mut found {
        package.deps.retain(|d| names.contains(d));
    }
    Ok(found)
}

/// The packages that depend on the given ones, directly or through others.
///
/// **The blast radius.** The packages themselves are not in the answer: the
/// caller says which packages changed, and this says who else has to be
/// checked because of it.
pub fn blast_radius(packages: &[Package], changed: &[String]) -> Vec<String> {
    let mut affected: Vec<String> = Vec::new();
    let mut frontier: Vec<String> = changed.to_vec();
    while let Some(name) = frontier.pop() {
        for package in packages {
            if package.deps.contains(&name)
                && !affected.contains(&package.name)
                && !changed.contains(&package.name)
            {
                affected.push(package.name.clone());
                frontier.push(package.name.clone());
            }
        }
    }
    affected.sort();
    affected
}

/// The packages that own the given files, by name.
///
/// The files are **relative to the workspace root**, which is the spelling git
/// uses.
///
/// A file at the root of the workspace belongs to no package, and a file in a
/// directory that is not a package's belongs to none either. The answer is
/// therefore a *subset* of the packages, and a subset is the honest answer
/// when the change touches the workspace's own files: those are checked by
/// everybody.
pub fn owners_of(packages: &[Package], files: &[PathBuf]) -> Vec<String> {
    let mut owners: Vec<String> = Vec::new();
    for file in files {
        for package in packages {
            if package.owns(file) && !owners.contains(&package.name) {
                owners.push(package.name.clone());
            }
        }
    }
    owners.sort();
    owners
}
