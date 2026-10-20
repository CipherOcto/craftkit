//! `cargo xtask ci` — everything, in the order that fails fastest.
//!
//! # The order
//!
//! ```text
//! fmt --check        a second, and a diff to read
//! assets             a second
//! lists              a second
//! layers             a second
//! web                a second
//! clippy -D warnings minutes
//! tests              minutes
//! wasm               minutes
//! ```
//!
//! The cheap file-reading gates come first so a formatting slip is reported in
//! a second rather than after a compile. The expensive gates come last.
//!
//! The wasm check is *not* run concurrently with the tests, and the reason is
//! worth stating because it looks like an oversight: both are cargo builds into
//! the same target directory, so the second one blocks on cargo's build lock
//! anyway. Running them at once would print interleaved output and make the
//! failure of either harder to read, for no time saved.
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

use crate::{assets, cargo, doctor, layers, lists, step, wasm, web};

/// One step of the gate.
pub struct Phase {
    pub name: &'static str,
    pub run: fn(&Path) -> Result<crate::Outcome, String>,
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
    Phase { name: "assets", run: assets::run },
    Phase { name: "lists", run: lists::run },
    Phase { name: "layers", run: layers::run },
    Phase { name: "web", run: web::run },
    Phase { name: "clippy", run: clippy },
    Phase { name: "tests", run: tests },
    Phase { name: "wasm", run: wasm::run },
];

/// `cargo fmt --check`.
pub fn fmt(root: &Path) -> Result<crate::Outcome, String> {
    step("fmt");
    cargo(&["fmt", "--all", "--check"], root)?;
    println!("   ok");
    Ok(crate::Outcome::Ran)
}

/// `cargo clippy` with warnings as errors, over every target.
///
/// `--all-targets` matters: the tests are where the interesting code is, and a
/// gate that does not look at them is not looking at most of the repository.
pub fn clippy(root: &Path) -> Result<crate::Outcome, String> {
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
pub fn tests(root: &Path) -> Result<crate::Outcome, String> {
    step("tests");
    cargo(&["test", "--workspace", "--profile", "ci"], root)?;
    println!("   ok");
    Ok(crate::Outcome::Ran)
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
        match (phase.run)(root) {
            Ok(crate::Outcome::Ran) => ran.push(phase.name),
            Ok(crate::Outcome::Skipped) => skipped.push(phase.name),
            Err(e) => return Err(format!("`{}` failed\n\n{e}", phase.name)),
        }
    }

    println!("{}", summary(started.elapsed().as_secs_f64(), &ran, &skipped));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phases_are_the_documented_ones_and_come_first() {
        let names: Vec<&str> = PHASES.iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            vec!["fmt", "assets", "lists", "layers", "web", "clippy", "tests", "wasm"],
            "the order is chosen so the cheapest gates run first, and `tests` and `wasm` must \
             still be in it: they used to be called after the loop, where deleting the call was \
             invisible — `ci` reported them as run on a build that ran neither"
        );
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
            !line.contains("— fmt, assets, lists, layers, web, clippy, tests, wasm."),
            "a skipped phase must not be listed as one that ran: {line}"
        );
        assert!(line.contains("skipped, not passed: wasm"), "{line}");
    }

    #[test]
    fn every_phase_reports_an_outcome_rather_than_barely_succeeding() {
        // A gate that could not say whether it ran is a gate `ci` cannot report
        // honestly, which is the failure the line above is about.
        fn returns_an_outcome(f: fn(&Path) -> Result<crate::Outcome, String>) {
            let _ = f;
        }
        for phase in PHASES {
            returns_an_outcome(phase.run);
        }
    }

    #[test]
    fn every_named_gate_exists_as_a_command() {
        // The failure this catches: `ci` runs a check, the usage text forgets
        // it, and an agent reads the usage text.
        let usage = crate::USAGE;
        for phase in PHASES {
            if phase.name == "fmt" || phase.name == "clippy" {
                continue; // part of "ci"'s line, not commands of their own
            }
            assert!(usage.contains(phase.name), "`{}` is not in the usage text", phase.name);
        }
        assert!(usage.contains("wasm"));
        assert!(usage.contains("doctor"));
    }

    #[test]
    fn no_phase_is_named_after_a_command_that_does_not_exist() {
        let usage = crate::USAGE;
        for command in ["layers", "assets", "lists", "web", "wasm", "doctor", "ci"] {
            assert!(usage.contains(command), "`{command}` is missing from the usage text");
        }
    }
}
