//! `cargo xtask doctor` — can this machine build this project?
//!
//! Every other gate here reports a *mistake*. This one reports the machine. It
//! exists because the failure modes it catches are the ones that waste the most
//! time: a toolchain older than the workspace's declared minimum, a target not
//! installed, a `wasm-bindgen` CLI whose version does not match the crate.
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
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
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

/// What the machine can tell us about the tool that produces the browser bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bundler {
    /// The CLI matches the crate the workspace resolves; the bundle can be built.
    Ready,
    /// It cannot, and this is exactly why.
    Blocked(String),
}

/// Decide **once**, for every caller, whether `wasm-bindgen` can produce a bundle.
///
/// `cli` is the version the installed CLI reported, or `None` when it is not
/// installed. `crate_version` is what the workspace resolves, or `None` when
/// that could not be read out of `Cargo.lock`.
///
/// It lives here rather than in each caller because `doctor` and
/// `cargo xtask bundle` both need the answer, and two copies of one policy is
/// how they came to disagree: when the crate version was unreadable, `bundle`
/// read that as "no blocker" while `doctor` filed it as `info`.
///
/// The case that matters is the second `None`. **"I could not read the version
/// I need to compare against" is not "they match."** There is no comparison
/// there to be right about, so this answers `Blocked` — the same rule
/// `Outcome::Skipped` and `live-window-check.sh`'s exit 77 exist to keep: a
/// check that could not run does not report that it passed.
pub fn bundler_status(cli: Option<&str>, crate_version: Option<&str>) -> Bundler {
    let Some(cli) = cli else {
        return Bundler::Blocked(
            "wasm-bindgen-cli is not installed (cargo install wasm-bindgen-cli --version \
             <the wasm-bindgen crate version>)"
                .to_string(),
        );
    };
    let Some(want) = crate_version else {
        return Bundler::Blocked(format!(
            "the wasm-bindgen crate version could not be read from Cargo.lock, so the CLI {cli} \
             cannot be checked against it"
        ));
    };
    match compare_versions(cli, want) {
        Some(std::cmp::Ordering::Equal) => Bundler::Ready,
        Some(_) => Bundler::Blocked(format!(
            "the CLI is {cli} but the wasm-bindgen crate is {want}; wasm-bindgen refuses a bundle \
             whose schema versions differ"
        )),
        None => Bundler::Blocked(format!("could not compare the CLI version {cli} with {want}")),
    }
}

/// The version string in `wasm-bindgen --version`, if the CLI is installed.
///
/// Only the last word is a version: the command prints `wasm-bindgen 0.2.129`,
/// and handing the whole line to the comparator makes it refuse — which the
/// first run of this proved the hard way.
pub fn installed_bundler_version(root: &Path) -> Option<String> {
    let out = command("wasm-bindgen", &["--version"], root).ok().filter(|o| o.ok())?;
    Some(out.stdout.trim().rsplit(' ').next().unwrap_or_default().to_string())
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

/// The `wasm-bindgen` crate version the workspace resolves to, if it does.
pub fn wasm_bindgen_crate_version(root: &Path) -> Option<String> {
    let lock = std::fs::read_to_string(root.join("Cargo.lock")).ok()?;
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() != "name = \"wasm-bindgen\"" {
            continue;
        }
        // The version line is the next one that starts with "version".
        return lines
            .find(|l| l.trim_start().starts_with("version"))
            .and_then(|l| l.split('"').nth(1))
            .map(str::to_string);
    }
    None
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
    match command("wasm-bindgen", &["--version"], root) {
        Ok(out) if out.ok() => {
            let cli = out.stdout.trim().rsplit(' ').next().unwrap_or_default().to_string();
            // The same decision `cargo xtask bundle` makes, by the same
            // function, so the two can no longer answer differently. It used to
            // be written twice, and when the crate version was unreadable this
            // half called it `info` while the other half called it fine.
            match bundler_status(Some(&cli), wasm_bindgen_crate_version(root).as_deref()) {
                Bundler::Ready => {}
                Bundler::Blocked(reason) => findings.push(Finding {
                    // Degraded, not blocking: nothing in `ci` runs this tool.
                    // Only the final bundling step needs it, and a report that
                    // cries blocking about the one thing that is not blocking
                    // trains people to ignore the severity.
                    severity: Severity::Degraded,
                    subject: "wasm-bindgen".to_string(),
                    message: reason,
                    fix: Some(
                        "cargo install wasm-bindgen-cli --version <the crate version> --force".to_string(),
                    ),
                }),
            }
        }
        Ok(out) => findings.push(Finding {
            severity: Severity::Degraded,
            subject: "wasm-bindgen".to_string(),
            message: format!("wasm-bindgen --version failed:\n{}", out.all()),
            fix: Some("cargo install wasm-bindgen-cli".to_string()),
        }),
        Err(_) => findings.push(Finding {
            severity: Severity::Degraded,
            subject: "wasm-bindgen".to_string(),
            message: "not installed; `cargo xtask wasm` will build the .wasm but cannot bundle it"
                .to_string(),
            fix: Some("cargo install wasm-bindgen-cli".to_string()),
        }),
    }

    // The optional tool that parses the service worker.
    if command("node", &["--version"], Path::new(".")).is_ok_and(|out| out.ok()) {
        findings.push(Finding {
            severity: Severity::Info,
            subject: "node".to_string(),
            message: "installed; `cargo xtask web` parses the service worker".to_string(),
            fix: None,
        });
    } else {
        findings.push(Finding {
            severity: Severity::Degraded,
            subject: "node".to_string(),
            message: "not installed; the service worker is checked by reading it, not by parsing it"
                .to_string(),
            fix: Some("install node, or read apps/craftkit-web/sw.js yourself".to_string()),
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

    /// The whole point of `bundler_status` living in one place: the case where the
    /// workspace version cannot be read used to mean "fine" to `cargo xtask
    /// bundle` and "info" to `doctor`. It is not fine — it is unknown, and an
    /// unknown is not a match.
    #[test]
    fn bundler_status_blocks_when_the_crate_version_cannot_be_read() {
        match bundler_status(Some("0.2.89"), None) {
            Bundler::Blocked(reason) => {
                assert!(reason.contains("could not be read"), "{reason}");
                // The CLI version belongs in the sentence: it is what the reader
                // has to go and fix.
                assert!(reason.contains("0.2.89"), "{reason}");
            }
            Bundler::Ready => panic!("an unverifiable CLI was accepted as ready"),
        }
    }

    #[test]
    fn bundler_status_blocks_when_the_cli_is_missing_and_says_what_to_do() {
        match bundler_status(None, Some("0.2.129")) {
            Bundler::Blocked(reason) => {
                assert!(reason.contains("not installed"), "{reason}");
                assert!(reason.contains("cargo install"), "{reason}");
            }
            Bundler::Ready => panic!("a missing CLI was accepted as ready"),
        }
    }

    #[test]
    fn bundler_status_is_ready_only_when_the_cli_matches_the_crate() {
        assert_eq!(bundler_status(Some("0.2.129"), Some("0.2.129")), Bundler::Ready);
        assert!(matches!(bundler_status(Some("0.2.89"), Some("0.2.129")), Bundler::Blocked(_)));
        // Unparseable on either side is unknown, not equal.
        assert!(matches!(bundler_status(Some("nightly"), Some("0.2.129")), Bundler::Blocked(_)));
    }

    #[test]
    fn the_workspace_declares_its_minimum_and_it_is_the_documented_one() {
        let root = crate::root().expect("root");
        assert_eq!(declared_msrv(&root).as_deref(), Some(MSRV));
    }

    #[test]
    fn the_wasm_bindgen_crate_version_can_be_read_from_the_lock_file() {
        let root = crate::root().expect("root");
        let version = wasm_bindgen_crate_version(&root);
        assert!(version.as_deref().is_some_and(|v| v.starts_with("0.2.")), "{version:?}");
    }

    #[test]
    fn a_missing_lock_file_is_none_not_a_panic() {
        assert_eq!(wasm_bindgen_crate_version(Path::new("/nonexistent")), None);
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
