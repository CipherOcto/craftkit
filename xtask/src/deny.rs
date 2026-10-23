//! `cargo xtask deny` — the dependency policy: licences, bans, sources, advisories.
//!
//! # Why this is a command and not a line in the workflow
//!
//! The policy was a check that lived in `.github/workflows/ci.yml` and nowhere
//! else: the `policy` job ran `cargo deny check` in the YAML, the workflow's own
//! header said *"the gates live in `cargo xtask`, not in this file"*, and no
//! command in the tree had the policy's name. That is §169's defect — a gate
//! with no name — in the one file that declares the opposite.
//!
//! Worse, it could not have passed. It was measured, on the tree as it stood,
//! with the real tool:
//!
//! ```text
//! advisories FAILED, bans FAILED, licenses FAILED, sources ok
//! ```
//!
//! Three of the four sections failed: one unmaintained advisory, fifty-one
//! "wildcard" dependencies that were all `{ workspace = true }`, and three
//! licence refusals. The rule and the tree had never been in the same place.
//! `KNOWN-GAPS.md` §173 has the whole measurement.
//!
//! # What this gate is, exactly
//!
//! It runs `cargo deny check` against `deny.toml`, and it has three rules:
//!
//! - **The policy does not hold is a failure.** Not a warning, not a note.
//! - **The tool is not installed is not a pass.** It is a skip, printed as a
//!   skip, with the command that would install it. A gate that cannot run says
//!   so; it never says "fine".
//! - **Where the tool is required, its absence is a failure.** In CI the
//!   policy job installs `cargo-deny` before calling this, so a machine that
//!   silently failed to install it must not turn the policy into a skip.
//!
//! # Why it is not a phase of `cargo xtask ci`
//!
//! `cargo deny check` needs the network and a licence database, and a gate that
//! fails for want of either gets switched off. That reason was already written
//! down, in the workflow, before this command existed; it is now written in the
//! tree, where the rule lives, and the workflow runs this command by name.
//!
//! What keeps that from being a check that runs only when somebody remembers
//! it is a rule in `cargo xtask lists`: **a command `ci` does not run must be
//! run by name in the workflow.** §173.

use std::path::Path;

use crate::{Outcome, command, skipped, step};

/// The tool the gate needs.
pub const TOOL: &str = "cargo-deny";

/// How to install it.
pub const INSTALL: &str = "cargo install cargo-deny --locked";

/// The policy file the gate reads.
pub const POLICY: &str = "deny.toml";

/// What the gate concluded about the dependency policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conclusion {
    /// The policy holds.
    Pass,
    /// The policy was not checked, and nothing here required it to be.
    ///
    /// Never a pass. A gate that could not run and a gate that found nothing
    /// wrong must not look the same.
    NotChecked,
    /// The policy was not checked, and it had to be: this is an error.
    Required,
    /// The tool ran and the policy does not hold.
    Failed(i32),
}

/// Decide what the gate concludes.
///
/// **Pure, and a function of its own, so that a test can pin it.** The rule
/// this holds is the one that matters: *a tool that is not there, and a tool
/// that failed, are both not a pass.* When this lived inside the code that ran
/// the check, nothing could check it — the lesson of §102, where a rule buried
/// in a function that runs the whole build could only be "tested" by the build
/// failing to compile.
///
/// `installed` is whether the tool is here; `required` is whether this machine
/// insists on it; `code` is the tool's exit code, or `None` when it could not
/// be run at all.
pub fn conclude(installed: bool, required: bool, code: Option<i32>) -> Conclusion {
    if !installed {
        return if required { Conclusion::Required } else { Conclusion::NotChecked };
    }
    match code {
        Some(0) => Conclusion::Pass,
        Some(code) => Conclusion::Failed(code),
        // The tool was there a moment ago and cannot be run now. That is a
        // failure, not a skip: something changed under the gate.
        None => Conclusion::Failed(-1),
    }
}

/// Is the tool installed on this machine?
pub fn installed(root: &Path) -> bool {
    match command(TOOL, &["--version"], root) {
        Ok(out) => out.ok(),
        Err(_) => false,
    }
}

/// Does this machine require the tool?
///
/// Under CI the policy is not optional: a job that installs `cargo-deny` and
/// then cannot run it has failed to install it, and saying "skipped" would
/// turn a broken job into a green one.
pub fn required() -> bool {
    match std::env::var("CI") {
        Ok(value) => {
            let value = value.trim();
            !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
        }
        Err(_) => false,
    }
}

/// The gate.
pub fn run(root: &Path) -> Result<Outcome, String> {
    step("deny");
    if !installed(root) {
        return match conclude(false, required(), None) {
            Conclusion::Required => Err(format!("{TOOL} is required here and is not installed ({INSTALL})")),
            _ => {
                skipped("the dependency policy", &format!("{TOOL} is not installed ({INSTALL})"));
                Ok(Outcome::Skipped)
            }
        };
    }

    let out = command("cargo", &["deny", "--log-level", "error", "check"], root)?;
    match conclude(true, required(), Some(out.code)) {
        Conclusion::Pass => {
            println!("   ok");
            Ok(Outcome::Ran)
        }
        Conclusion::Failed(code) => Err(format!(
            "the dependency policy ({POLICY}) does not hold: cargo deny check exited {code}\n\n{}",
            out.all()
        )),
        // `installed` was true, so `conclude` cannot answer `Required` or
        // `NotChecked`. Written out rather than left to a panic: a gate that
        // crashes on an impossible branch is a gate that cannot be trusted with
        // the one branch nobody has tested.
        Conclusion::Required => Err("the dependency policy was not checked".to_string()),
        Conclusion::NotChecked => Err("the dependency policy was not checked".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_policy_that_holds_is_a_pass() {
        assert_eq!(conclude(true, false, Some(0)), Conclusion::Pass);
    }

    #[test]
    fn a_policy_that_does_not_hold_is_a_failure_and_not_a_skip() {
        // The failure this catches: the gate reported a broken policy as
        // "skipped", which reads in a CI log like a machine that did not have
        // the tool — and is the difference between a green job and a red one.
        assert_eq!(conclude(true, false, Some(1)), Conclusion::Failed(1));
        assert_ne!(conclude(true, false, Some(1)), Conclusion::NotChecked);
        assert_ne!(conclude(true, false, Some(1)), Conclusion::Pass);
    }

    #[test]
    fn a_tool_that_is_not_installed_is_not_a_pass() {
        // The failure this catches: a machine without cargo-deny reporting a
        // green policy. The policy was never checked.
        assert_eq!(conclude(false, false, None), Conclusion::NotChecked);
        assert_ne!(conclude(false, false, None), Conclusion::Pass);
    }

    #[test]
    fn a_tool_that_is_required_and_missing_is_an_error() {
        assert_eq!(conclude(false, true, None), Conclusion::Required);
        assert_ne!(conclude(false, true, None), Conclusion::NotChecked);
        assert_ne!(conclude(false, true, None), Conclusion::Pass);
    }

    #[test]
    fn a_tool_that_cannot_be_run_at_all_is_a_failure() {
        // It was there a moment ago and cannot be run now: something changed
        // under the gate, and a skip would hide that.
        assert_eq!(conclude(true, false, None), Conclusion::Failed(-1));
    }

    #[test]
    fn the_policy_and_the_tool_are_named_in_one_place() {
        // The rule and its list travel together: the file the tool reads is
        // named here, so a reviewer can see which policy this gate applies.
        assert_eq!(POLICY, "deny.toml");
        assert!(INSTALL.contains(TOOL), "{INSTALL}");
    }
}
