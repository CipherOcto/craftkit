//! `cargo xtask doctor` — can this machine build this project?
//!
//! Every other gate here reports a *mistake*. This one reports the machine. It
//! exists because the failure modes it catches are the ones that waste the most
//! time: a toolchain older than the workspace's declared minimum, a target not
//! installed, and a Node that is missing when a check needs to parse a script.
//!
//! Each finding is one of three kinds, and the difference matters:
//!
//! - **blocking** — a gate will fail, or a build will not start;
//! - **degraded** — something works worse than it should;
//! - **informational** — worth knowing, not worth changing.
//!
//! Nothing here is ever silently fixed, because every fix is a decision about
//! somebody's machine.

use std::path::Path;

use crate::wasm;
use crate::{command, step};

/// What kind of problem it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Will stop a build or a gate.
    Blocking,
    /// Works, but less well than expected.
    Degraded,
    /// Worth knowing.
    Info,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Blocking => "blocking",
            Severity::Degraded => "degraded",
            Severity::Info => "info",
        }
    }
}

/// The minimum toolchain this workspace supports, from `Cargo.toml`.
pub const MSRV: &str = "1.93";

/// The `rust-version` declared in the workspace manifest.
pub fn declared_msrv(root: &Path) -> Option<String> {
    let manifest = crate::tree::read_text(root.join("Cargo.toml")).ok()?;
    manifest.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix("rust-version")
            .and_then(|rest| rest.trim_start().strip_prefix('='))
            .map(|value| value.trim().trim_matches('"').to_string())
    })
}

/// Compare two dotted version strings.
///
/// Returns `None` when either cannot be read, because "I could not tell" is not
/// "it is fine" and must not be reported as a pass.
pub fn compare_versions(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let parse = |text: &str| -> Option<Vec<u32>> {
        text.split('.')
            .map(|part| {
                let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
                digits.parse::<u32>().ok()
            })
            .collect()
    };
    Some(parse(a)?.cmp(&parse(b)?))
}

/// One line of the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    pub subject: String,
    pub message: String,
    /// What to do about it, when there is something to do.
    pub fix: Option<String>,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {:<18} {}", self.severity.label(), self.subject, self.message)?;
        if let Some(fix) = &self.fix {
            write!(f, "\n{:>27}→ {fix}", "")?;
        }
        Ok(())
    }
}

/// Gather the findings. Kept separate from printing so it is testable.
pub fn examine(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();

    // The toolchain.
    match command("rustc", &["--version"], root) {
        Ok(out) if out.ok() => {
            let line = out.stdout.trim().to_string();
            let version =
                line.split_whitespace().nth(1).map(|v| v.split('-').next().unwrap_or(v).to_string());
            match (version, declared_msrv(root)) {
                (Some(have), Some(want)) => match compare_versions(&have, &want) {
                    Some(order) if order.is_lt() => findings.push(Finding {
                        severity: Severity::Blocking,
                        subject: "rustc".to_string(),
                        message: format!("{have} is older than the workspace minimum {want}"),
                        fix: Some(format!("rustup update stable  # or install {want} and above")),
                    }),
                    // "Could not compare" is reported, never treated as a pass.
                    None => findings.push(Finding {
                        severity: Severity::Degraded,
                        subject: "rustc".to_string(),
                        message: format!("could not compare {have} with the declared minimum {want}"),
                        fix: None,
                    }),
                    _ => {}
                },
                (_, None) => findings.push(Finding {
                    severity: Severity::Info,
                    subject: "rustc".to_string(),
                    message: format!("{line}; the workspace declares no rust-version"),
                    fix: None,
                }),
                (None, _) => findings.push(Finding {
                    severity: Severity::Degraded,
                    subject: "rustc".to_string(),
                    message: format!("could not read a version out of “{line}”"),
                    fix: None,
                }),
            }
        }
        Ok(out) => findings.push(Finding {
            severity: Severity::Blocking,
            subject: "rustc".to_string(),
            message: format!("rustc --version failed:\n{}", out.all()),
            fix: None,
        }),
        Err(e) => findings.push(Finding {
            severity: Severity::Blocking,
            subject: "rustc".to_string(),
            message: e,
            fix: None,
        }),
    }

    // The wasm target, and the CLI that post-processes the bundle.
    if !wasm::target_installed() {
        findings.push(Finding {
            severity: Severity::Degraded,
            subject: "wasm32".to_string(),
            message: format!("the {} target is not installed", wasm::TARGET),
            fix: Some(format!("rustup target add {}", wasm::TARGET)),
        });
    }
    // The Node that parses the browser door's script.
    //
    // The v2 tree has no `wasm-bindgen`: the browser door is a plain
    // WebAssembly module, and the tree's own bundler writes it. So there is
    // no CLI whose version has to match a crate's, and nothing to check but
    // Node.
    if command("node", &["--version"], Path::new(".")).is_ok_and(|out| out.ok()) {
        findings.push(Finding {
            severity: Severity::Info,
            subject: "node".to_string(),
            message: "installed; `cargo xtask web` parses the browser door's script".to_string(),
            fix: None,
        });
    } else {
        findings.push(Finding {
            severity: Severity::Degraded,
            subject: "node".to_string(),
            message: "not installed; the browser door's script is checked by reading it, not by parsing it"
                .to_string(),
            fix: Some("install node, or read apps/craftkit-wasm/browser-check.mjs yourself".to_string()),
        });
    }

    findings
}

/// The gate. Prints the report; fails only on a blocking finding.
pub fn run(root: &Path) -> Result<(), String> {
    step("doctor");
    let findings = examine(root);
    for finding in &findings {
        println!("   {finding}");
    }
    let blocking: Vec<&Finding> = findings.iter().filter(|f| f.severity == Severity::Blocking).collect();
    if blocking.is_empty() {
        println!("   nothing blocking");
        return Ok(());
    }
    Err(format!(
        "{} blocking finding(s) on this machine; the gates above will fail until they are fixed",
        blocking.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_numerically_not_lexically() {
        use std::cmp::Ordering;
        // The classic trap: "1.10" < "1.9" as text.
        assert_eq!(compare_versions("1.10.0", "1.9.0"), Some(Ordering::Greater));
        assert_eq!(compare_versions("1.95.0", "1.95.0"), Some(Ordering::Equal));
        assert_eq!(compare_versions("1.94.1", "1.95.0"), Some(Ordering::Less));
    }

    #[test]
    fn a_pre_release_counts_as_its_release() {
        use std::cmp::Ordering;
        assert_eq!(compare_versions("1.96.0-nightly", "1.96.0"), Some(Ordering::Equal));
    }

    #[test]
    fn an_unreadable_version_is_none_not_equal() {
        assert_eq!(compare_versions("banana", "1.95"), None);
        assert_eq!(compare_versions("", "1.95"), None);
        assert_eq!(compare_versions("1.95", "banana"), None);
    }

    #[test]
    fn the_workspace_declares_its_minimum_and_it_is_the_documented_one() {
        let root = crate::root().expect("root");
        assert_eq!(declared_msrv(&root).as_deref(), Some(MSRV));
    }

    #[test]
    fn a_missing_lock_file_is_none_not_a_panic() {
        assert_eq!(declared_msrv(Path::new("/nonexistent")), None);
    }

    #[test]
    fn nothing_on_this_machine_blocks_a_gate() {
        // Not "the machine has everything": just "nothing blocking". A gate
        // that fails on a working machine is a gate people disable.
        let root = crate::root().expect("root");
        let blocking: Vec<Finding> =
            examine(&root).into_iter().filter(|f| f.severity == Severity::Blocking).collect();
        assert!(
            blocking.is_empty(),
            "{}",
            blocking.iter().map(Finding::to_string).collect::<Vec<_>>().join("\n")
        );
    }

    #[test]
    fn the_report_names_a_subject_and_says_something_about_it() {
        let root = crate::root().expect("root");
        let findings = examine(&root);
        assert!(!findings.is_empty(), "doctor reported nothing at all");
        for finding in &findings {
            assert!(!finding.subject.is_empty());
            assert!(!finding.message.is_empty(), "{finding}");
        }
    }

    #[test]
    fn severities_are_ordered_so_the_worst_prints_first() {
        assert!(Severity::Blocking < Severity::Degraded);
        assert!(Severity::Degraded < Severity::Info);
        let mut unsorted = vec![Severity::Info, Severity::Blocking, Severity::Degraded];
        unsorted.sort();
        assert_eq!(unsorted, vec![Severity::Blocking, Severity::Degraded, Severity::Info]);
    }

    #[test]
    fn a_finding_with_a_fix_says_it_on_the_next_line() {
        let finding = Finding {
            severity: Severity::Blocking,
            subject: "rustc".to_string(),
            message: "too old".to_string(),
            fix: Some("rustup update stable".to_string()),
        };
        let text = finding.to_string();
        assert!(text.contains("blocking"), "{text}");
        assert!(text.contains("rustup update stable"), "{text}");
    }
}
