//! `cargo xtask ci` — everything, in the order that fails fastest.
//!
//! # The order
//!
//! ```text
//! fmt --check        a second, and a diff to read
//! anchors            a second
//! assets             a second
//! lists              a second
//! layers             a second
//! web                a second
//! clippy -D warnings minutes
//! tests              minutes
//! fuzz               seconds
//! wasm               minutes
//! ```
//!
//! The cheap file-reading gates come first so a formatting slip is reported in
//! a second rather than after a compile. The expensive gates come last.
//!
//! The **anchors** phase is second because it protects the instrument every
//! other measurement comes from: `scripts/mutation-test-gates.sh` finds a moved
//! anchor when the case that uses it is reached, which is up to twenty-five
//! minutes into a run. Checked up front, it is a second. §184.
//!
//! The wasm check is *not* run concurrently with the tests, and the reason is
//! worth stating because it looks like an oversight: both are cargo builds into
//! the same target directory, so the second one blocks on cargo's build lock
//! anyway. Running them at once would print interleaved output and make the
//! failure of either harder to read, for no time saved.
//!
//! The fuzz phase sits between the tests and the wasm check for two reasons.
//! It needs the CLI the tests have just built, so it must come after them; and
//! a crash it finds is worth more than a wasm build that takes four minutes to
//! find out about, so it must come before.
//!
//! # What it stops at
//!
//! The first failure. A CI run that reports twelve problems after a formatting
//! slip is a CI run people learn to skim. One problem, named clearly, with the
//! file and line, is one somebody fixes.
//!
//! # What is never a pass
//!
//! A gate that could not run. `wasm` skips with a reason when the target is not
//! installed; `web` skips its parse when node is missing. A skip is printed as
//! a skip, in the middle of the run, where it cannot be mistaken for a pass.

use std::path::Path;
use std::time::Instant;

use crate::{anchors, assets, cargo, cleanroom, doctor, fuzz, layers, lists, step, wasm, web};

/// One step of the gate.
pub struct Phase {
    pub name: &'static str,
    pub run: fn(&Path, &[String]) -> Result<crate::Outcome, String>,
}

/// The phases, in order. Public so a test can assert the order and the set.
///
/// **One list, and `run` iterates this one.** That is the whole point of it
/// being a single constant: an earlier version ran the cheap gates from `PHASES`
/// and called the tests and the wasm check separately, after the loop. Deleting
/// that one line left `cargo xtask ci` printing "ci green — …, tests, wasm" and
/// exiting 0 without running either. Nothing noticed, because the closing line
/// was a string literal rather than a record of what had happened. Found by
/// removing the call and reading the output; the closing line is now built from
/// the phases that ran, and `the_closing_line_names_what_actually_ran` pins it.
pub const PHASES: &[Phase] = &[
    Phase { name: "fmt", run: fmt },
    Phase { name: "anchors", run: anchors::run },
    Phase { name: "assets", run: assets::run },
    Phase { name: "cleanroom", run: cleanroom::run },
    Phase { name: "lists", run: lists::run },
    Phase { name: "layers", run: layers::run },
    Phase { name: "web", run: web::run },
    Phase { name: "clippy", run: clippy },
    Phase { name: "tests", run: tests },
    Phase { name: "fuzz", run: fuzz::command },
    Phase { name: "wasm", run: wasm::run },
];

/// `cargo fmt --check`.
pub fn fmt(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    step("fmt");
    cargo(&["fmt", "--all", "--check"], root)?;
    println!("   ok");
    Ok(crate::Outcome::Ran)
}

/// `cargo clippy` with warnings as errors, over every target.
///
/// `--all-targets` matters: the tests are where the interesting code is, and a
/// gate that does not look at them is not looking at most of the repository.
pub fn clippy(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    step("clippy");
    cargo(
        &[
            "clippy",
            "--workspace",
            "--all-targets",
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
    println!("   ok");
    Ok(crate::Outcome::Ran)
}

/// The tests. Its own phase, so `ci` can say whether it ran.
///
/// The wasm check is the next phase rather than the tail of this one, and the
/// ordering is the reason they are sequential: both are cargo builds into the
/// same target directory, so the second blocks on cargo's build lock anyway.
/// Running them at once would print interleaved output and make the failure of
/// either harder to read, for no time saved. [`PHASES`] is a sequence, so
/// running them in order is what this file does by construction.
///
/// **One package at a time, and each one named.** `cargo test --workspace`
/// runs every package's tests and says nothing until they are all done — so a
/// run that hangs is a hang with **no evidence** about where it was. That is
/// not hypothetical: **measured** on the Windows runner, this phase ran for
/// **92 minutes** and produced no output at all, and the job was cancelled
/// with nothing to read. A run that names its package says where it is
/// before it hangs. §224.
///
/// **And the binaries are run directly, so the *test* is named too.** Cargo
/// captures a test binary's output and prints it when the binary exits, so a
/// test that hangs prints nothing — the package name says which package, and
/// nothing says which test. Running the compiled binary ourselves hands it our
/// stdout, and the harness prints each test's name as it goes.
pub fn tests(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    step("tests");
    for package in workspace_packages(root)? {
        println!("   {}", package.name);
        test_package(root, &package)?;
    }
    println!("   ok");
    Ok(crate::Outcome::Ran)
}

/// One package of the workspace, and where its tests live.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Package {
    /// The package name, as cargo calls it.
    name: String,
    /// Cargo's own id for the package, which is how a build message says which
    /// package it is about.
    id: String,
    /// The directory the package lives in. Cargo runs a package's test
    /// binaries with that directory as the working directory, and a test that
    /// opens a file by a relative path depends on it.
    dir: std::path::PathBuf,
    /// Whether the package has a library target, and therefore doc tests.
    has_lib: bool,
}

/// The packages of the workspace, in the order cargo lists them.
///
/// Read from cargo's own metadata, so a new crate is tested the moment it is
/// created and not the moment somebody remembers.
fn workspace_packages(root: &Path) -> Result<Vec<Package>, String> {
    let out = crate::command("cargo", &["metadata", "--no-deps", "--format-version", "1", "--offline"], root)
        .or_else(|_| crate::command("cargo", &["metadata", "--no-deps", "--format-version", "1"], root))?;
    if !out.ok() {
        return Err(format!("cargo metadata failed\n\n{}", out.all()));
    }
    let value: serde_json::Value =
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
        let dir = std::path::Path::new(manifest)
            .parent()
            .ok_or_else(|| format!("{name} has no directory in cargo metadata"))?
            .to_path_buf();
        let has_lib = entry
            .get("targets")
            .and_then(|t| t.as_array())
            .map(|targets| {
                targets.iter().any(|t| {
                    t.get("kind")
                        .and_then(|k| k.as_array())
                        .map(|kinds| kinds.iter().any(|k| k.as_str() == Some("lib")))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        found.push(Package { name, id, dir, has_lib });
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}

/// Run one package's tests, naming the test that hangs.
///
/// **The test binaries are run directly, not through `cargo test`.** Cargo
/// captures a test binary's output and prints it when the binary *exits*, so
/// a test that hangs produces no output at all — the run says nothing about
/// where it stopped. Running the compiled binary ourselves gives it our
/// stdout, and the test harness prints each test's name as it goes, so the
/// last name printed is the test that hung. §224.
///
/// Doc tests still go through cargo: they are compiled by cargo and there is
/// no binary to run ourselves.
fn test_package(root: &Path, package: &Package) -> Result<(), String> {
    let built = crate::command(
        "cargo",
        &["test", "--profile", "ci", "--package", &package.name, "--no-run", "--message-format", "json"],
        root,
    )?;
    if !built.ok() {
        return Err(format!("building the tests of {} failed\n\n{}", package.name, built.all()));
    }
    let binaries = test_binaries(&built.stdout, &package.id);
    if binaries.is_empty() {
        return Err(format!(
            "cargo built no test binary for {} — the tree's tests would not have run",
            package.name
        ));
    }
    for binary in binaries {
        let path = if binary.is_absolute() { binary } else { root.join(&binary) };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        println!("      {name}");
        let status = std::process::Command::new(&path)
            .current_dir(&package.dir)
            // Cargo sets this for a test binary; the xtask's own tests read it
            // at run time, so a binary run outside cargo needs it too.
            .env("CARGO_MANIFEST_DIR", &package.dir)
            .status()
            .map_err(|e| format!("could not run the test binary {}: {e}", path.display()))?;
        if !status.success() {
            return Err(format!("the tests of {} failed ({name})", package.name));
        }
    }
    if package.has_lib {
        cargo(&["test", "--profile", "ci", "--package", &package.name, "--doc"], root)?;
    }
    Ok(())
}

/// The test binaries cargo built, from what it printed.
///
/// **From cargo's JSON, not from its wording.** Cargo's human output prints an
/// `Executable ...` line per test binary, and that line is not a contract: the
/// tree's own CI read those lines, and on the Windows runner cargo printed
/// **none of them** — the phase said "no test binary" on a build that had
/// produced one. §224.
///
/// The JSON is a contract: every build produces one `compiler-artifact`
/// message per artifact, and a test artifact says where its executable is.
/// Only artifacts of **this** package are taken, and only those built with the
/// **test profile** on — a package's *binary* is also an artifact with an
/// executable, and running the application itself is not a test.
fn test_binaries(stdout: &str, package_id: &str) -> Vec<std::path::PathBuf> {
    let mut found: Vec<std::path::PathBuf> = Vec::new();
    for line in stdout.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("reason").and_then(|r| r.as_str()) != Some("compiler-artifact") {
            continue;
        }
        if value.get("package_id").and_then(|p| p.as_str()) != Some(package_id) {
            continue;
        }
        // A **test** artifact is one built with the test profile switched on:
        // that is the harness, not the library or the program itself.
        let is_test =
            value.get("profile").and_then(|p| p.get("test")).and_then(|t| t.as_bool()).unwrap_or(false);
        if !is_test {
            continue;
        }
        let Some(executable) = value.get("executable").and_then(|e| e.as_str()) else {
            continue;
        };
        found.push(std::path::PathBuf::from(executable));
    }
    found.sort();
    found.dedup();
    found
}

/// The closing line of `ci`.
///
/// Built from the phases that ran rather than written out by hand, because the
/// hand-written version was a claim about the code instead of a record of it:
/// it listed `tests` and `wasm` on a build where neither had run. A phase that
/// could not run is named as skipped, never listed as one that ran.
pub fn summary(elapsed_secs: f64, ran: &[&str], skipped: &[&str]) -> String {
    let mut line = format!("\nci green in {elapsed_secs:.1}s — {}", ran.join(", "));
    if !skipped.is_empty() {
        line.push_str(&format!("\n   skipped, not passed: {}", skipped.join(", ")));
    }
    line.push('.');
    line
}

/// The whole gate.
pub fn run(root: &Path) -> Result<(), String> {
    let started = Instant::now();

    // doctor runs first and only reports: it is not a gate, and a machine that
    // cannot build should say so before three minutes of compiler output.
    if let Err(e) = doctor::run(root) {
        println!("\n{e}");
    }

    let mut ran: Vec<&str> = Vec::with_capacity(PHASES.len());
    let mut skipped: Vec<&str> = Vec::new();

    // The first failure. See "What it stops at" above.
    for phase in PHASES {
        match (phase.run)(root, &[]) {
            Ok(outcome) => record(outcome, phase.name, &mut ran, &mut skipped),
            Err(e) => return Err(format!("`{}` failed\n\n{e}", phase.name)),
        }
    }

    println!("{}", summary(started.elapsed().as_secs_f64(), &ran, &skipped));
    Ok(())
}

/// Record what a phase did.
///
/// A phase that could not run is **skipped**, and a skipped phase is not one
/// that ran: the closing line says which is which, and a phase that is
/// counted as having run when it did not is a gate that lies about itself.
///
/// This is a function of its own so that the rule can be pinned by a test:
/// the loop it came from was three lines inside a function that runs the whole
/// build, and nothing could check what it did. §102.
fn record<'a>(outcome: crate::Outcome, name: &'a str, ran: &mut Vec<&'a str>, skipped: &mut Vec<&'a str>) {
    match outcome {
        crate::Outcome::Ran => ran.push(name),
        crate::Outcome::Skipped => skipped.push(name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phases_are_the_documented_ones_and_come_first() {
        let names: Vec<&str> = PHASES.iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            vec![
                "fmt",
                "anchors",
                "assets",
                "cleanroom",
                "lists",
                "layers",
                "web",
                "clippy",
                "tests",
                "fuzz",
                "wasm",
            ],
            "the order is chosen so the cheapest gates run first, and `tests` and `wasm` must \
             still be in it: they used to be called after the loop, where deleting the call was \
             invisible — `ci` reported them as run on a build that ran neither"
        );
    }

    /// The fuzz phase runs after the tests, because it fuzzes the binary the
    /// tests built.
    ///
    /// It is a phase and not a separate command somebody has to remember:
    /// a check that runs only when somebody remembers it is the same defect
    /// as a check that does not exist (§169).
    #[test]
    fn the_fuzz_phase_runs_after_the_tests_and_before_the_wasm_check() {
        let names: Vec<&str> = PHASES.iter().map(|p| p.name).collect();
        let tests = names.iter().position(|n| *n == "tests").expect("tests is a phase");
        let fuzz = names.iter().position(|n| *n == "fuzz").expect("fuzz is a phase");
        let wasm = names.iter().position(|n| *n == "wasm").expect("wasm is a phase");
        assert!(tests < fuzz, "the fuzzer runs before the CLI it fuzzes is built");
        assert!(fuzz < wasm, "a crash is worth more than a four-minute wasm build");
    }

    #[test]
    fn the_closing_line_names_what_actually_ran() {
        // The failure this catches: the line used to be a literal listing every
        // phase, so it kept claiming `tests, wasm` after the calls that ran
        // them had been deleted.
        let all: Vec<&str> = PHASES.iter().map(|p| p.name).collect();
        assert!(summary(0.0, &all, &[]).contains("tests"));
        assert!(summary(0.0, &all, &[]).contains("wasm"));

        // A phase that could not run is named as skipped, and is not in the
        // list of what ran.
        let without_wasm: Vec<&str> = PHASES.iter().filter(|p| p.name != "wasm").map(|p| p.name).collect();
        let line = summary(0.0, &without_wasm, &["wasm"]);
        assert!(
            !line.contains("— fmt, assets, cleanroom, lists, layers, web, clippy, tests, wasm."),
            "a skipped phase must not be listed as one that ran: {line}"
        );
        assert!(line.contains("skipped, not passed: wasm"), "{line}");
    }

    /// A phase that could not run is not counted as one that ran.
    ///
    /// §102: the rule lived inside the loop that runs the whole build, where
    /// nothing could check it, and the mutation that broke it was reported as
    /// caught only because it left a variable unused — which the clippy phase
    /// refuses, so the *build* failed and the rule was never tested at all.
    ///
    /// The rule is a function now, and this is its test.
    #[test]
    fn a_phase_that_could_not_run_is_not_counted_as_one_that_ran() {
        let mut ran: Vec<&str> = Vec::new();
        let mut skipped: Vec<&str> = Vec::new();
        record(crate::Outcome::Skipped, "wasm", &mut ran, &mut skipped);
        assert_eq!(ran, Vec::<&str>::new(), "a skipped phase was counted as one that ran");
        assert_eq!(skipped, vec!["wasm"]);

        record(crate::Outcome::Ran, "tests", &mut ran, &mut skipped);
        assert_eq!(ran, vec!["tests"]);
        assert_eq!(skipped, vec!["wasm"], "a skipped phase was forgotten");
    }

    #[test]
    fn every_phase_reports_an_outcome_rather_than_barely_succeeding() {
        // A gate that could not say whether it ran is a gate `ci` cannot report
        // honestly, which is the failure the line above is about.
        fn returns_an_outcome(f: fn(&Path, &[String]) -> Result<crate::Outcome, String>) {
            let _ = f;
        }
        for phase in PHASES {
            returns_an_outcome(phase.run);
        }
    }

    #[test]
    fn every_phase_of_ci_is_named_in_the_usage_text() {
        // The failure this catches: `ci` runs a check, the usage text forgets
        // it, and an agent reads the usage text. Every phase is named by
        // `ci`'s own line, so `fmt` and `clippy` are named too — they were
        // exceptions to this test until that line listed them.
        let usage = crate::usage();
        for phase in PHASES {
            assert!(usage.contains(phase.name), "`{}` is not in the usage text", phase.name);
        }
    }

    #[test]
    fn every_command_is_in_the_usage_text() {
        // The failure this catches: a command exists and runs, and neither
        // the usage text nor the crate's own table mentions it. `bundle` ran,
        // and was in neither, for as long as the tree has had it. §169.
        let usage = crate::usage();
        for command in crate::COMMANDS {
            assert!(usage.contains(command.name), "`{}` is missing from the usage text", command.name);
        }
    }
}
