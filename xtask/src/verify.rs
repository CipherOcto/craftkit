//! `cargo xtask verify` — check the tree, at the size of the change.
//!
//! # The problem this solves
//!
//! The gate runs the whole tree: fifty-one crates, formatted, linted and
//! tested. That is the right check **before a release** and the wrong check
//! **while a change is being made**, because a change to one battery was
//! being verified by rebuilding every battery in the tree.
//!
//! **Measured** on this repository's own history: the workspace test ran
//! **959 times** and took **26.8 hours** — and **448** of those runs, longer
//! than half a minute, held **94 %** of that time. The same battery's tests
//! run in **0.8 seconds**.
//!
//! # The three scopes
//!
//! | the flag | what is checked |
//! |---|---|
//! | `--package <name>` | that package, and everything that depends on it |
//! | `--changed` | the packages the working tree changed, and everything that depends on them |
//! | `--all`, or nothing | every package — the gate's own scope |
//!
//! **The dependants are part of the answer.** A change to `craftkit-sdk` is a
//! change to every battery, and a check that verified only the SDK would be
//! a check that missed the breakage it was looking for. The blast radius is
//! computed from the manifest graph (see [`crate::packages::blast_radius`]),
//! and it is the *only* definition of "what this change can break" the tree
//! has.
//!
//! # What it reports
//!
//! The scope it chose, the packages it checked, and — in words — the packages
//! it **did not** check. A scoped check that does not say what it left out is
//! a check that reads as a whole check, and the tree's rule is that a check
//! which could not run is never reported as one that passed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::packages::{Package, blast_radius, owners_of, workspace_packages};
use crate::{cargo, command, step};

/// What the command was asked to check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// Every package in the workspace.
    All,
    /// These packages, and everything that depends on them.
    Packages(Vec<String>),
    /// The packages the working tree changed, and everything that depends on
    /// them.
    Changed,
}

/// The scope the flags asked for, and the packages it names.
pub fn parse_scope(args: &[String]) -> Result<Scope, String> {
    let mut named: Vec<String> = Vec::new();
    let mut changed = false;
    let mut all = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--all" => all = true,
            "--changed" => changed = true,
            "--package" => {
                index += 1;
                let Some(name) = args.get(index) else {
                    return Err("--package needs the name of a package".to_string());
                };
                named.push(name.clone());
            }
            other => return Err(format!("unknown flag `{other}`: try --package, --changed or --all")),
        }
        index += 1;
    }
    if all && (changed || !named.is_empty()) {
        return Err("--all is the whole tree; it cannot be combined with a narrower scope".to_string());
    }
    if changed && !named.is_empty() {
        return Err("--changed and --package are two scopes; pick one".to_string());
    }
    if changed {
        return Ok(Scope::Changed);
    }
    if !named.is_empty() {
        return Ok(Scope::Packages(named));
    }
    Ok(Scope::All)
}

/// The files the working tree changed, against `HEAD`.
///
/// **Both the tracked and the untracked**, because a new file is as much a
/// change as an edited one — and a new file in a battery that the scope
/// missed is exactly the mistake this command exists to prevent.
///
/// When there is no git — a tarball, a fresh checkout with no history — the
/// answer is **every file**, and the scope falls back to the whole tree. The
/// tree says so rather than pretending it found nothing.
pub fn changed_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let tracked = command("git", &["diff", "--name-only", "HEAD"], root)?;
    let staged = command("git", &["diff", "--name-only", "--cached"], root)?;
    let status = command("git", &["status", "--porcelain"], root)?;
    if !tracked.ok() || !staged.ok() || !status.ok() {
        return Err("git could not say what changed; the scope falls back to the whole tree".to_string());
    }
    let mut files: Vec<PathBuf> = Vec::new();
    for line in tracked.stdout.lines().chain(staged.stdout.lines()) {
        let name = line.trim();
        if !name.is_empty() {
            files.push(PathBuf::from(name));
        }
    }
    for line in status.stdout.lines() {
        // A porcelain line is "XY path", and the path may be quoted.
        let name = line.get(3..).unwrap_or("").trim().trim_matches('"');
        if name.is_empty() || name.starts_with("??") && name.len() == 2 {
            continue;
        }
        if line.starts_with("??") {
            let path = PathBuf::from(name);
            if !files.contains(&path) {
                files.push(path);
            }
        }
    }
    Ok(files)
}

/// The packages a scope names, and the ones it leaves out.
pub struct Resolution {
    /// The packages to check.
    pub selected: Vec<String>,
    /// The packages that are not checked, and why they were left out.
    pub skipped: Vec<String>,
}

/// The packages a scope resolves to.
///
/// A package that is asked for by name and is **not in the workspace** is an
/// error, not a silent nothing: a scope that names a package that does not
/// exist would check nothing at all and say "ok".
pub fn resolve(scope: &Scope, packages: &[Package], changed: &[PathBuf]) -> Result<Resolution, String> {
    let all: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();
    match scope {
        Scope::All => Ok(Resolution { selected: all.clone(), skipped: Vec::new() }),
        Scope::Packages(named) => {
            for name in named {
                if !all.contains(name) {
                    return Err(format!(
                        "`{name}` is not a package of this workspace; the packages are: {}",
                        all.join(", ")
                    ));
                }
            }
            let mut selected: Vec<String> = named.clone();
            for name in blast_radius(packages, named) {
                if !selected.contains(&name) {
                    selected.push(name);
                }
            }
            selected.sort();
            let mut skipped: Vec<String> = all.iter().filter(|a| !selected.contains(a)).cloned().collect();
            skipped.sort();
            Ok(Resolution { selected, skipped })
        }
        Scope::Changed => {
            let owners = owners_of(packages, changed);
            if owners.is_empty() {
                // Nothing in the workspace changed: the workspace's own files
                // may have. Checking the whole tree is the honest answer.
                return Ok(Resolution { selected: all.clone(), skipped: Vec::new() });
            }
            let mut selected = owners.clone();
            for name in blast_radius(packages, &owners) {
                if !selected.contains(&name) {
                    selected.push(name);
                }
            }
            selected.sort();
            let mut skipped: Vec<String> = all.iter().filter(|a| !selected.contains(a)).cloned().collect();
            skipped.sort();
            Ok(Resolution { selected, skipped })
        }
    }
}

/// Run the command.
pub fn run(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    let scope = parse_scope(args)?;
    let packages = workspace_packages(root)?;
    let changed = match scope {
        Scope::Changed => match changed_files(root) {
            Ok(files) => files,
            Err(e) => {
                crate::skipped("the changed files", &e);
                Vec::new()
            }
        },
        _ => Vec::new(),
    };
    let resolution = resolve(&scope, &packages, &changed)?;

    step("verify");
    println!("   scope: {}", describe(&scope));
    for name in &resolution.selected {
        println!("   check: {name}");
    }
    if !resolution.skipped.is_empty() {
        println!("   not checked ({}): {}", resolution.skipped.len(), resolution.skipped.join(", "));
    }

    let selected: Vec<&Package> = packages.iter().filter(|p| resolution.selected.contains(&p.name)).collect();

    for package in &selected {
        cargo(&["fmt", "--package", &package.name, "--check"], root)?;
        cargo(
            &[
                "clippy",
                "--package",
                &package.name,
                "--all-targets",
                "--all-features",
                "--profile",
                "ci",
                "--",
                "-D",
                "clippy::all",
                "-D",
                "warnings",
            ],
            root,
        )?;
    }
    build_the_shared_libraries(root, &selected)?;
    for package in &selected {
        crate::ci::test_package(root, package)?;
    }

    println!("   ok — {} of {} packages checked", resolution.selected.len(), packages.len());
    Ok(crate::Outcome::Ran)
}

/// The scope, in words.
fn describe(scope: &Scope) -> String {
    match scope {
        Scope::All => "the whole tree".to_string(),
        Scope::Packages(named) => format!("{} and what depends on it", named.join(", ")),
        Scope::Changed => "the packages the working tree changed, and what depends on them".to_string(),
    }
}

/// Build every package in the scope whose library is also a shared library.
///
/// **Before the tests, and for the reason the gate gives:** cargo writes a
/// package's `.so` when the package is **built**, and not when it is used as
/// another package's dependency. The fixtures are dev-dependencies, so
/// `cargo test --package craftkit-au` never writes
/// `libcraftkit_au_fixture.so`, and the host's test opens a file that is not
/// there.
fn build_the_shared_libraries(root: &Path, packages: &[&Package]) -> Result<(), String> {
    let mut names: Vec<&str> = packages.iter().filter(|p| p.is_cdylib).map(|p| p.name.as_str()).collect();
    if names.is_empty() {
        return Ok(());
    }
    names.sort_unstable();
    let mut args: Vec<&str> = vec!["build", "--profile", "ci"];
    for name in &names {
        args.push("--package");
        args.push(name);
    }
    cargo(&args, root)
}

/// The names of the packages in the scope, for the report.
pub fn names_of(packages: &[Package], selected: &[String]) -> BTreeSet<String> {
    packages.iter().filter(|p| selected.contains(&p.name)).map(|p| p.name.clone()).collect()
}
