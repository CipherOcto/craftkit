//! The **scope** of the verification, and the report it makes.
//!
//! The scope is the whole point of `cargo xtask verify`: a change to one
//! battery is checked by that battery and by what depends on it, and the
//! packages it leaves out are **named** — because a scoped check that does
//! not say what it skipped reads as a whole check.
//!
//! These tests are about the decision, not about cargo: they build the
//! package graph by hand, so they say what the scope *is* without a build,
//! and they fail loudly when the rule changes.

use std::path::{Path, PathBuf};

use xtask::packages::{Package, blast_radius, owners_of};
use xtask::verify::{Scope, parse_scope, resolve};

/// A package of the graph, for the tests to use.
fn package(name: &str, deps: &[&str]) -> Package {
    Package {
        name: name.to_string(),
        id: format!("path+file:///tree/{name}#0.1.0"),
        dir: PathBuf::from(format!("/tree/crates/{name}")),
        rel_dir: PathBuf::from(format!("crates/{name}")),
        has_lib: true,
        is_cdylib: false,
        deps: deps.iter().map(|d| d.to_string()).collect(),
    }
}

/// The tree the tests think about: an SDK, two batteries and an application.
fn the_tree() -> Vec<Package> {
    vec![
        package("craftkit-sdk", &[]),
        package("craftkit-plugin-grid", &["craftkit-sdk"]),
        package("craftkit-plugin-sound", &["craftkit-sdk"]),
        package("craftkit-studio", &["craftkit-sdk", "craftkit-plugin-grid", "craftkit-plugin-sound"]),
    ]
}

#[test]
fn the_flags_name_the_scope() {
    assert_eq!(parse_scope(&[]).expect("no flags is the whole tree"), Scope::All);
    assert_eq!(parse_scope(&["--all".to_string()]).expect("--all is the whole tree"), Scope::All);
    assert_eq!(parse_scope(&["--changed".to_string()]).expect("--changed is its own scope"), Scope::Changed);
    assert_eq!(
        parse_scope(&["--package".to_string(), "craftkit-plugin-grid".to_string()])
            .expect("--package names one package"),
        Scope::Packages(vec!["craftkit-plugin-grid".to_string()])
    );
}

#[test]
fn two_scopes_at_once_are_refused() {
    let both = ["--changed".to_string(), "--package".to_string(), "x".to_string()];
    assert!(parse_scope(&both).is_err(), "--changed and --package are two scopes");
    let everything = ["--all".to_string(), "--changed".to_string()];
    assert!(parse_scope(&everything).is_err(), "--all cannot be narrowed");
    let nameless = ["--package".to_string()];
    assert!(parse_scope(&nameless).is_err(), "--package needs a name");
    let unknown = ["--everything".to_string()];
    assert!(parse_scope(&unknown).is_err(), "an unknown flag is refused");
}

#[test]
fn a_change_carries_what_depends_on_it() {
    let tree = the_tree();
    // The SDK is under the grid, the sound and the studio.
    assert_eq!(
        blast_radius(&tree, &["craftkit-sdk".to_string()]),
        vec![
            "craftkit-plugin-grid".to_string(),
            "craftkit-plugin-sound".to_string(),
            "craftkit-studio".to_string()
        ]
    );
    // The grid is under the studio only.
    assert_eq!(
        blast_radius(&tree, &["craftkit-plugin-grid".to_string()]),
        vec!["craftkit-studio".to_string()]
    );
    // The studio is under nothing: it is an application.
    assert!(blast_radius(&tree, &["craftkit-studio".to_string()]).is_empty());
}

#[test]
fn the_scope_is_the_change_and_what_depends_on_it() {
    let tree = the_tree();
    let resolution =
        resolve(&Scope::Changed, &tree, &[PathBuf::from("crates/craftkit-plugin-grid/src/lib.rs")])
            .expect("the grid changed");
    assert_eq!(resolution.selected, vec!["craftkit-plugin-grid".to_string(), "craftkit-studio".to_string()]);
    // And the report names what was left out.
    assert_eq!(resolution.skipped, vec!["craftkit-plugin-sound".to_string(), "craftkit-sdk".to_string()]);
}

#[test]
fn a_package_that_is_not_there_is_an_error_not_a_nothing() {
    let tree = the_tree();
    let resolution = resolve(&Scope::Packages(vec!["craftkit-plugin-grid".to_string()]), &tree, &[]);
    assert!(resolution.is_ok(), "a package of the tree resolves");
    let missing = resolve(&Scope::Packages(vec!["craftkit-plugin-nope".to_string()]), &tree, &[]);
    assert!(missing.is_err(), "a package that is not there is refused, not ignored");
}

#[test]
fn a_change_to_the_workspace_itself_checks_the_whole_tree() {
    let tree = the_tree();
    // A file that belongs to no package — the workspace's own manifest.
    let resolution =
        resolve(&Scope::Changed, &tree, &[PathBuf::from("Cargo.toml"), PathBuf::from("README.md")])
            .expect("the workspace's own files");
    assert_eq!(resolution.selected.len(), tree.len(), "everything is checked");
    assert!(resolution.skipped.is_empty(), "nothing is left out");
}

#[test]
fn a_file_belongs_to_the_package_that_holds_it() {
    let tree = the_tree();
    let owners = owners_of(
        &tree,
        &[
            PathBuf::from("crates/craftkit-plugin-grid/src/lib.rs"),
            PathBuf::from("crates/craftkit-plugin-sound/src/lib.rs"),
            PathBuf::from("README.md"),
        ],
    );
    assert_eq!(owners, vec!["craftkit-plugin-grid".to_string(), "craftkit-plugin-sound".to_string()]);
    assert!(!owners.iter().any(|o| o == "craftkit-sdk"), "the README is not the SDK's");
}

#[test]
fn a_package_owns_the_files_under_its_directory() {
    let tree = the_tree();
    let grid = tree.iter().find(|p| p.name == "craftkit-plugin-grid").expect("the grid");
    assert!(grid.owns(Path::new("crates/craftkit-plugin-grid/src/lib.rs")));
    assert!(!grid.owns(Path::new("crates/craftkit-plugin-sound/src/lib.rs")));
}
