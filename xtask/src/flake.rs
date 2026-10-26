//! `cargo xtask flake` — the one-off failures, run again and again.
//!
//! # What this is for
//!
//! **Two tests have failed once each, and never again.** One in
//! `craftkit-engine`'s library tests, one in `craftkit-ui`'s. Neither failure
//! was captured, so neither has a cause; §207 ran the engine's tests **150**
//! more times and §216 ran the UI's in four configurations, and **0** of them
//! failed. The tree's own item 42 is still open because the two ways of
//! looking for the cause are both expensive:
//!
//! - the **loop** — run the tests until one fails and capture the name — is
//!   what would find it, and the owner ruled it out on a shared machine: a
//!   test that takes all the CPU for more than five minutes needs approval
//!   first;
//! - the **repetition count** in the `ci` profile — cheap, and it is what
//!   this is.
//!
//! **And it runs on the runner, not on a person's machine.** The owner's rule
//! is about *local* processes: a machine that takes all the CPU is somebody
//! else's. A CI runner has nobody's machine to take, so the heavy half of
//! this item belongs there, and this command is what the runner runs.
//!
//! # What it does
//!
//! **One number of repetitions, in one place.** [`REPETITIONS`], and the two
//! suites in [`SUITES`]. The tree's rule is that a bound has one name and one
//! home, so the count is not a flag and not an argument: there is nothing to
//! pass differently on two machines.
//!
//! **A failing run is kept and named.** The output of every failing
//! repetition is written to `flake-out/` and the report says where. §216: a
//! report that a reader can lose is not evidence, and the failure that
//! started this item was lost exactly that way.
//!
//! # What it decides
//!
//! **One failure is a flake, and two are a bug.**
//!
//! The tree's rule is that **CI must never fail at random**. A job that
//! failed the build on the first rare failure would be a job that fails at
//! random, and a job that fails at random is a job people learn to ignore.
//! So:
//!
//! - **no failure** in [`REPETITIONS`] repetitions: nothing to say;
//! - **one** failure of a test: reported, loudly, with its output kept — and
//!   the command **succeeds**, because a single failure is the flake this
//!   item is about;
//! - **the same test failing twice**, or any failure that named no test at
//!   (a crash), **twice**: the command **fails**. That is not a flake any
//!   more, and a build that is red for it is a build that is right.
//!
//! So the job can only go red when it has something to say, and it says it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How many times each suite is run.
///
/// **One number, in one place.** §216's lesson: a measurement is not a
/// repetition — the count is the bound, and the bound has one name.
pub const REPETITIONS: u32 = 30;

/// The suites that failed once each, and never again.
///
/// The engine's failure was in `--lib`; the UI's was in `--lib`. Both are
/// named here rather than guessed at: a repetition of the wrong suite is a
/// measurement of the wrong thing.
pub const SUITES: &[(&str, &str)] = &[("craftkit-engine", "--lib"), ("craftkit-ui", "--lib")];

/// Where the output of a failing repetition is kept.
///
/// A scratch directory, like the fuzzer's: it is not part of the tree, and it
/// is gitignored.
pub const OUTPUT_DIR: &str = "flake-out";

/// The name given to a failure that named no test.
///
/// A run that dies before it can say which test failed is a **crash**, and a
/// crash twice is as reproducible as a failure twice.
const UNNAMED: &str = "(the run failed and named no test)";

/// Run the command.
pub fn run(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    crate::step("flake");
    let mut failures: BTreeMap<String, usize> = BTreeMap::new();
    let mut kept: Vec<PathBuf> = Vec::new();

    for (suite, filter) in SUITES {
        println!("   {suite} {filter}: {REPETITIONS} repetitions");
        for round in 1..=REPETITIONS {
            let out = run_once(root, suite, filter)?;
            if out.ok() {
                continue;
            }
            let names = failing_tests(&out);
            let names = if names.is_empty() { vec![UNNAMED.to_string()] } else { names };
            for name in &names {
                *failures.entry(name.clone()).or_insert(0) += 1;
            }
            let file = keep(root, suite, round, &out)?;
            kept.push(file.clone());
            println!("      repetition {round}: {} failed — {}", names.join(", "), file.display());
        }
    }

    report(&failures, &kept)
}

/// Run one repetition of one suite.
fn run_once(root: &Path, suite: &str, filter: &str) -> Result<crate::Output, String> {
    let args = ["test", "--profile", "ci", "--package", suite, filter];
    // `nice -n 15`, like every other build this tree runs, so a repetition
    // never takes a machine that somebody else is using.
    let nice_args = ["-n", "15", "cargo", "test", "--profile", "ci", "--package", suite, filter];
    crate::command("nice", &nice_args, root).or_else(|_| crate::command("cargo", &args, root))
}

/// The tests a failed run names.
///
/// Cargo prints `test <name> ... FAILED` for each one. A run that died
/// before it could say names none, and that is a different thing — a crash —
/// so the caller gives it a name of its own.
fn failing_tests(output: &crate::Output) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for line in output.stdout.lines().chain(output.stderr.lines()) {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("test ") else { continue };
        let Some((name, tail)) = rest.split_once(" ... ") else { continue };
        if tail.trim() == "FAILED" {
            let name = name.trim();
            if !name.is_empty() && !names.iter().any(|n| n == name) {
                names.push(name.to_string());
            }
        }
    }
    names
}

/// Keep the output of a failing repetition, and say where it is.
///
/// §216: the output of a failure is the only copy of what the failing run
/// said, and a copy that is deleted on success is a copy nobody can read
/// when it matters.
fn keep(root: &Path, suite: &str, round: u32, output: &crate::Output) -> Result<PathBuf, String> {
    let dir = root.join(OUTPUT_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let path = dir.join(format!("{suite}-{round}.log"));
    let text = format!("{}\n{}", output.stdout, output.stderr);
    std::fs::write(&path, text).map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(path)
}

/// Say what happened, and decide whether the build goes red.
///
/// **One failure is a flake; the same test twice is a bug.** The tree's rule
/// is that CI must never fail at random, so a single failure is reported and
/// the command succeeds.
fn report(failures: &BTreeMap<String, usize>, kept: &[PathBuf]) -> Result<crate::Outcome, String> {
    let repeated: Vec<(&String, &usize)> = failures.iter().filter(|(_, count)| **count >= 2).collect();

    if !repeated.is_empty() {
        let mut message = String::from("a test failed more than once, so this is not a flake:\n");
        for (name, count) in repeated {
            message.push_str(&format!("   {name}: {count} failures\n"));
        }
        if !kept.is_empty() {
            let files: Vec<String> = kept.iter().map(|p| p.display().to_string()).collect();
            message.push_str(&format!(
                "\nThe output of every failure is in {} — it was kept, because a failing \
                 gate throws its evidence away.\n",
                files.join(" and ")
            ));
        }
        return Err(message);
    }

    if failures.is_empty() {
        println!("   no failure in {REPETITIONS} repetitions of each suite");
    } else {
        println!("   one failure each, and no test failed twice — so these are flakes:");
        for (name, count) in failures {
            println!("      {name}: {count}");
        }
        let files: Vec<String> = kept.iter().map(|p| p.display().to_string()).collect();
        println!("      the output is in {}", files.join(" and "));
        println!("      a single failure does not fail this job: CI must never fail at random.");
    }
    Ok(crate::Outcome::Ran)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A failed run names the tests that failed.
    #[test]
    fn a_failed_run_names_its_tests() {
        let output = crate::Output {
            code: 101,
            stdout: "test one::two ... ok\ntest three::four ... FAILED\n".to_string(),
            stderr: "test five::six ... FAILED\n".to_string(),
            kept: Vec::new(),
        };
        assert_eq!(failing_tests(&output), vec!["three::four", "five::six"]);
    }

    /// A run that died before it could say names nothing.
    #[test]
    fn a_run_that_died_names_nothing() {
        let output = crate::Output {
            code: 101,
            stdout: "running 12 tests\n".to_string(),
            stderr: "thread 'main' panicked\n".to_string(),
            kept: Vec::new(),
        };
        assert!(failing_tests(&output).is_empty());
    }

    /// One failure is a flake, and the command succeeds.
    #[test]
    fn one_failure_is_a_flake_and_does_not_fail_the_job() {
        let mut failures = BTreeMap::new();
        failures.insert("a::test".to_string(), 1);
        let outcome = report(&failures, &[]).expect("a single failure is not a failure of the job");
        assert_eq!(outcome, crate::Outcome::Ran);
    }

    /// The same test twice is a bug, and the command fails.
    #[test]
    fn the_same_test_twice_fails_the_job() {
        let mut failures = BTreeMap::new();
        failures.insert("a::test".to_string(), 2);
        let error = report(&failures, &[]).expect_err("two failures are not a flake");
        assert!(error.contains("a::test"), "{error}");
    }

    /// No failure at all is a pass, and says so.
    #[test]
    fn no_failure_is_a_pass() {
        let outcome = report(&BTreeMap::new(), &[]).expect("no failure is not a failure");
        assert_eq!(outcome, crate::Outcome::Ran);
    }
}
