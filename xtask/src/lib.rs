//! Workspace tooling: `cargo xtask <command>`.
//!
//! This crate is the project's only build tool, and it is written in the
//! project's own house style: no `unwrap`, no `panic`, errors as `Result`. A
//! gate that panics on a malformed manifest is a gate that lies.
//!
//! # Why the checks are code and not CI YAML
//!
//! Every check here has the same shape — read files, compare against a table
//! that lives in this repository, report what disagrees. Doing that in code
//! means:
//!
//! - the table is versioned with the code it checks, so a change to the layering
//!   rules is reviewed as a diff;
//! - the same check runs identically on a laptop, in CI, and on a build machine
//!   an agent is driving;
//! - the rules are unit-tested, so a gate cannot quietly stop checking anything.
//!
//! # The commands
//!
//! | Command | What it protects |
//! |---|---|
//! | [`anchors`] | the mutation script's anchors and its file list are in the tree, its labels and its texts are literals the shell cannot change, its header names every way to run it, and it checks itself before it starts |
//! | [`layers`] | the dependency layering, and the UI boundary |
//! | [`assets`] | "every icon is drawn in code; every file is attributed" |
//! | [`cleanroom`] | "no product's name is in the tree outside prose" |
//! | [`lists`] | the shared lines and sorted lists many changes touch |
//! | [`web`] | the browser entry page and its service worker |
//! | [`wasm`] | everything below the apps still builds for `wasm32` |
//! | [`deny`] | the dependency policy: licences, bans, sources, advisories |
//! | [`fuzz`] | the design-file reader, fuzzed: broken files in, crashes out |
//! | [`flake`] | the two suites that failed once, run again and again, and the failure named |
//! | [`bundle`] | the browser bundle is built, and written |
//! | [`doctor`] | the machine is set up to build this at all |
//! | [`ci`] | all of the above, in order, then the tests |
//!
//! A gate that cannot run on this machine is reported as skipped with a reason,
//! never as passed. `doctor` is how you find out which.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod anchors;
pub mod assets;
pub mod ci;
pub mod cleanroom;
pub mod deny;
pub mod doctor;
pub mod flake;
pub mod fuzz;
pub mod layers;
pub mod lists;
pub mod temp;
pub mod tree;
pub mod wasm;
pub mod web;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

/// A command `cargo xtask` accepts.
///
/// The name, what it does, and what it runs. **One list**, so the usage text,
/// this module's own table and the gate that checks the prose cannot disagree
/// about what the commands are. §169.
///
/// The arguments are passed in rather than read from the environment,
/// because a command with a flag is a command somebody can find: the
/// environment is for configuration that has to cross a process boundary,
/// not for the knobs of a tool.
pub struct Subcommand {
    /// What the user types.
    pub name: &'static str,
    /// What it does, in one line.
    pub what: &'static str,
    /// What it runs.
    pub run: fn(&Path, &[String]) -> Result<Outcome, String>,
}

/// Every command, in the order the usage text lists them.
///
/// `doctor` and `ci` are here too, so that the list of what `cargo xtask`
/// accepts is one list.
pub const COMMANDS: &[Subcommand] = &[
    Subcommand {
        name: "anchors",
        what: "the mutation script's anchors and its file list are in the tree, and it checks itself before it starts",
        run: anchors::run,
    },
    Subcommand {
        name: "layers",
        what: "enforce the crate dependency layering (docs/architecture.md)",
        run: layers::run,
    },
    Subcommand {
        name: "assets",
        what: "every icon is drawn in code and every file asset is attributed",
        run: assets::run,
    },
    Subcommand {
        name: "cleanroom",
        what: "no product's name is in the tree outside prose (docs/clean-room.md)",
        run: cleanroom::run,
    },
    Subcommand {
        name: "lists",
        what: "the shared lines and sorted lists many changes add to stay sorted",
        run: lists::run,
    },
    Subcommand { name: "web", what: "check the browser entry page and the service worker", run: web::run },
    Subcommand {
        name: "wasm",
        what: "every crate below the apps still builds for wasm32-unknown-unknown",
        run: wasm::run,
    },
    Subcommand {
        name: "deny",
        what: "the dependency policy: licences, bans, sources and advisories (deny.toml)",
        run: deny::run,
    },
    Subcommand {
        name: "fuzz",
        what: "fuzz the design-file reader: broken files in, crashes out",
        run: fuzz::command,
    },
    Subcommand {
        name: "flake",
        what: "run the two suites that failed once again and again, and name the failure (it repeats, so CI runs it in its own job)",
        run: flake::run,
    },
    Subcommand {
        name: "bundle",
        what: "build the browser bundle (it writes files; `ci` does not run it)",
        run: wasm::bundle,
    },
    Subcommand {
        name: "doctor",
        what: "report whether this machine can build the project, and what is missing",
        run: run_doctor,
    },
    Subcommand {
        name: "ci",
        what: "fmt --check, anchors, assets, cleanroom, lists, layers, web, clippy -D warnings, tests, fuzz, wasm",
        run: run_ci,
    },
];

/// The commands `ci` does not run, and why each one is not a phase of it.
///
/// A command that checks something and is not a phase of `ci` is a check that
/// runs only when somebody remembers it — a gate that is real but never runs,
/// which is the same defect as a gate that does not exist. §169.
///
/// **The default is the other way round.** A command that is not in this list
/// is a gate, and `ci` runs it. So a new command is a gate until somebody
/// says otherwise, in one line, here, with a reason a reviewer can read. The
/// alternative — a flag on each command — is a flag nobody has to justify,
/// and a gate can be silenced by forgetting one character.
///
/// **And a command in this list must be run by name in the workflow.** That is
/// what makes the exception safe rather than a hole: the check that `ci` does
/// not run is still run, on every push, by the job that owns it. `cargo xtask
/// lists` holds this: a command listed here that the workflow never runs is a
/// check that only happens when somebody remembers, and it is reported as
/// such. §173.
pub const NOT_GATES: &[(&str, &str)] = &[
    ("bundle", "it writes files into the tree"),
    ("doctor", "it reports on the machine, not on the code"),
    ("deny", "it needs the network and a licence database; CI runs it by name"),
    ("ci", "it is the runner"),
    (
        "flake",
        "it repeats a test that failed once, so it is a measurement and not a gate; CI runs it in its own job",
    ),
];

/// Whether `ci` runs this command.
pub fn is_gate(name: &str) -> bool {
    !NOT_GATES.iter().any(|(not, _)| *not == name)
}

/// `doctor` as a command: it reports, and its failure is the command's failure.
fn run_doctor(root: &Path, _args: &[String]) -> Result<Outcome, String> {
    doctor::run(root)?;
    Ok(Outcome::Ran)
}

/// `ci` as a command: the whole gate.
fn run_ci(root: &Path, _args: &[String]) -> Result<Outcome, String> {
    ci::run(root)?;
    Ok(Outcome::Ran)
}

/// The usage text, printed by `cargo xtask --help`.
///
/// Generated from the command table, so it cannot forget a command: a command
/// that is not in the usage text is a command nobody finds.
pub fn usage() -> String {
    let mut out = String::from(
        "usage: cargo xtask <command>\n\n\
         The gates. Run `ci` before every commit; the rest are what it runs.\n\n",
    );
    for command in COMMANDS {
        out.push_str(&format!("  {:<10} {}\n", command.name, command.what));
    }
    out.push_str("\n  -h, --help  this message\n");
    out
}

/// Dispatch one `cargo xtask` invocation.
///
/// Returns the process exit code, so `main` is a single call and every command
/// gets the same error reporting.
pub fn run() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = match root() {
        Ok(root) => root,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    // The gates report `Outcome` so `ci` can tell a phase that ran from one that
    // was skipped. Standalone, both outcomes mean the same thing: no failure.
    let result = match args.first().map(String::as_str) {
        Some("-h" | "--help" | "help") | None => {
            print!("{}", usage());
            Ok(())
        }
        // The command table is the only list of what `cargo xtask` accepts, so
        // a command cannot exist without being in it, and the usage text is
        // written from it. §169.
        Some(other) => match COMMANDS.iter().find(|command| command.name == other) {
            Some(command) => (command.run)(&root, &args[1..]).map(drop),
            None => Err(format!("unknown command `{other}`\n\n{}", usage())),
        },
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("\nerror: {e}");
            ExitCode::FAILURE
        }
    }
}

/// The workspace root: the parent of the directory holding this crate.
///
/// Derived from this file's path rather than from the current directory, so
/// `cargo xtask` works from anywhere in the tree — including from a test that
/// runs with the crate directory as its working directory.
pub fn root() -> Result<PathBuf, String> {
    // CARGO_MANIFEST_DIR is <root>/xtask. It is set for `cargo run`, but not
    // necessarily for a test binary, so fall back to walking up from the
    // compiled executable as well.
    if let Some(dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let path = PathBuf::from(dir);
        if let Some(parent) = path.parent() {
            return Ok(parent.to_path_buf());
        }
    }
    let exe = std::env::current_exe().map_err(|e| format!("cannot find this executable: {e}"))?;
    // target/{profile}/deps/xtask-hash → target/{profile} → target → root
    for up in [3usize, 4, 5] {
        let mut path = exe.clone();
        for _ in 0..up {
            let Some(parent) = path.parent() else { break };
            path = parent.to_path_buf();
        }
        if path.join("Cargo.toml").is_file() && path.join("xtask").is_dir() {
            return Ok(path);
        }
    }
    Err("cannot work out the workspace root; run this through `cargo xtask`".to_string())
}

/// Run a command, returning its exit code and its output when it fails.
///
/// Output is captured so a failing gate can show the compiler's own words,
/// which is the difference between a report an agent can act on and one it has
/// to guess at.
#[derive(Debug)]
pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    /// The files the output was captured in, when they were **kept**.
    ///
    /// §216. A gate that fails keeps its capture files and names them, so
    /// that the evidence survives a reader who cut the report short — which is
    /// how a failure that happened once was lost once already.
    pub kept: Vec<PathBuf>,
}

impl Output {
    pub fn ok(&self) -> bool {
        self.code == 0
    }

    /// Everything the command said, for a failure report.
    pub fn all(&self) -> String {
        let mut s = String::new();
        if !self.stdout.trim().is_empty() {
            s.push_str(self.stdout.trim());
        }
        if !self.stderr.trim().is_empty() {
            if !s.is_empty() {
                s.push('\n');
            }
            s.push_str(self.stderr.trim());
        }
        if s.is_empty() {
            s = format!("(exit code {}) with no output", self.code);
        }
        if !self.kept.is_empty() {
            let files: Vec<String> = self.kept.iter().map(|p| p.display().to_string()).collect();
            s.push_str(&format!(
                "\n\nThe output is also in {} — it was kept, because the gate failed.",
                files.join(" and ")
            ));
        }
        s
    }
}

/// Run `program` with `args` in `dir`, capturing both streams.
///
/// # Why a file and not a pipe
///
/// `wait_with_output` is the obvious way to do this and it is wrong here. In
/// some sandboxed and containerised environments — including the one this
/// project was built in — a spawned child's pipe file descriptors are not
/// readable by the parent: `wait_with_output` returns **empty output and a
/// correct exit code**.
///
/// A gate built on that cannot tell "the compiler said nothing" from "I cannot
/// hear the compiler". It would report an empty error for a real failure, and
/// somebody would spend an afternoon looking for a bug that is in the report.
/// So each stream goes to a temporary file, which is always readable, and the
/// file is removed afterwards. One temporary file per invocation is a cheap
/// price for a gate that can be believed.
pub fn command(program: &str, args: &[&str], dir: &Path) -> Result<Output, String> {
    let capture = Capture::new(program)?;
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .stdin(std::process::Stdio::null())
        .stdout(capture.stdout.try_clone().map_err(|e| format!("{e}"))?)
        .stderr(capture.stderr.try_clone().map_err(|e| format!("{e}"))?)
        .status()
        .map_err(|e| {
            let _ = capture.cleanup();
            format!("could not run `{program}`: {e}")
        })?;

    let stdout = capture.read(&capture.paths[0])?;
    let stderr = capture.read(&capture.paths[1])?;
    // **A failure keeps its evidence.** §216: the capture files are the only
    // copy of what the child said, and a report that a reader cut short —
    // `tail`, a pipe, a scroll-back that has gone — is a report that is
    // gone. So the files stay, and the failure names them.
    let code = status.code().unwrap_or(-1);
    let mut kept: Vec<PathBuf> = Vec::new();
    if code == 0 {
        let _ = capture.cleanup();
    } else {
        kept = capture.paths.to_vec();
    }

    Ok(Output { code, stdout, stderr, kept })
}

/// What a child did, when it was run with a time limit.
#[derive(Debug)]
pub struct TimedOutput {
    /// The exit code, or `None` when the child died by a signal.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    /// Whether the child had to be killed for running too long.
    pub timed_out: bool,
}

impl TimedOutput {
    /// Everything the child said, for a failure report.
    pub fn all(&self) -> String {
        let mut s = String::new();
        if !self.stdout.trim().is_empty() {
            s.push_str(self.stdout.trim());
        }
        if !self.stderr.trim().is_empty() {
            if !s.is_empty() {
                s.push('\n');
            }
            s.push_str(self.stderr.trim());
        }
        if s.is_empty() { format!("(exit code {}) with no output", self.code.unwrap_or(-1)) } else { s }
    }
}

/// Run `program` with `args`, and kill it if it runs for longer than `timeout`.
///
/// The same file-based capture as [`command`], for the same reason: a pipe
/// can be unreadable in a sandboxed environment, and a gate that cannot hear
/// the child cannot report what the child said.
///
/// The time limit is the reason this exists at all. A child that never
/// finishes is a thing a parent can see and a program cannot: `fuzz` needs
/// to tell a hang from a refusal, and only the outside can tell.
pub fn command_timed(
    program: &Path,
    args: &[String],
    dir: &Path,
    timeout: Duration,
    env: &[(String, String)],
) -> Result<TimedOutput, String> {
    let capture = Capture::new(&program.to_string_lossy())?;
    let mut child = Command::new(program)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(capture.stdout.try_clone().map_err(|e| format!("{e}"))?)
        .stderr(capture.stderr.try_clone().map_err(|e| format!("{e}"))?)
        .envs(env.iter().map(|(k, v)| (k, v)))
        .spawn()
        .map_err(|e| {
            let _ = capture.cleanup();
            format!("could not run `{}`: {e}", program.display())
        })?;

    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    timed_out = true;
                    break child.wait().map_err(|e| {
                        let _ = capture.cleanup();
                        format!("could not wait for `{}`: {e}", program.display())
                    })?;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(e) => {
                let _ = capture.cleanup();
                return Err(format!("could not wait for `{}`: {e}", program.display()));
            }
        }
    };

    let stdout = capture.read(&capture.paths[0])?;
    let stderr = capture.read(&capture.paths[1])?;
    let _ = capture.cleanup();

    Ok(TimedOutput { code: status.code(), stdout, stderr, timed_out })
}

/// Two temporary files, one per stream, removed on the way out.
struct Capture {
    stdout: std::fs::File,
    stderr: std::fs::File,
    paths: [std::path::PathBuf; 2],
}
impl Capture {
    fn new(program: &str) -> Result<Self, String> {
        // A counter rather than the program name alone: two invocations of the
        // same program must not share a file, and the tests run in parallel
        // within one process.
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let safe: String =
            program.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
        let dir = std::env::temp_dir();
        let paths = [
            dir.join(format!("craftkit-xtask-{safe}-{}-{n}.out", std::process::id())),
            dir.join(format!("craftkit-xtask-{safe}-{}-{n}.err", std::process::id())),
        ];
        let stdout = std::fs::File::create(&paths[0])
            .map_err(|e| format!("could not create {}: {e}", paths[0].display()))?;
        let stderr = std::fs::File::create(&paths[1])
            .map_err(|e| format!("could not create {}: {e}", paths[1].display()))?;
        Ok(Self { stdout, stderr, paths })
    }

    fn read(&self, path: &std::path::Path) -> Result<String, String> {
        crate::tree::read_text(path).map_err(|e| format!("could not read {}: {e}", path.display()))
    }

    fn cleanup(&self) -> std::io::Result<()> {
        for path in &self.paths {
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }
}

/// Run a cargo subcommand and fail with its output if it fails.
pub fn cargo(args: &[&str], dir: &Path) -> Result<(), String> {
    // `nice -n 15` so a gate run on a shared machine yields to real work. It
    // is advisory: if `nice` is missing, the build still runs.
    let mut nice_args = vec!["-n", "15", "cargo"];
    nice_args.extend_from_slice(args);
    let out = match command("nice", &nice_args, dir) {
        Ok(out) => out,
        Err(_) => command("cargo", args, dir)?,
    };
    if out.ok() { Ok(()) } else { Err(format!("cargo {} failed\n\n{}", args.join(" "), out.all())) }
}

/// Print a step heading, the way every gate in this project announces itself.
pub fn step(name: &str) {
    println!("\n── {name}");
}

/// What one gate did.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    /// It ran, and it passed.
    Ran,
    /// It could not run, and printed why. Never reported as a pass.
    Skipped,
}

/// Report a check that could not run.
///
/// Never reported as a pass. A gate skipped because its tool is missing and a
/// gate skipped because nothing was wrong must not look the same.
pub fn skipped(what: &str, why: &str) {
    println!("   skipped: {what} — {why}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_root_is_the_directory_holding_the_workspace_manifest() {
        let root = root().expect("finds the root");
        assert!(root.join("Cargo.toml").is_file(), "no manifest in {}", root.display());
        assert!(root.join("xtask").is_dir(), "no xtask in {}", root.display());
        assert!(root.join("crates").is_dir(), "no crates in {}", root.display());
    }

    #[test]
    fn the_root_is_an_absolute_path() {
        // A relative root would work exactly once, from the one directory that
        // happened to line up, which is the worst possible failure mode for a
        // gate: it passes locally and fails everywhere else.
        let root = root().expect("finds the root");
        assert!(root.is_absolute(), "{}", root.display());
    }

    #[test]
    fn a_failing_command_reports_its_own_words() {
        let out = command("sh", &["-c", "echo out; echo err >&2; exit 3"], Path::new(".")).expect("sh runs");
        assert!(!out.ok());
        assert_eq!(out.code, 3);
        let all = out.all();
        assert!(all.contains("out"), "{all}");
        assert!(all.contains("err"), "{all}");
    }

    /// A failing command leaves its output where a person can find it.
    ///
    /// §216. The capture files are the only copy of what the child said, and
    /// a report that a reader cut short — `tail`, a pipe, a scroll-back that
    /// has gone — is a report that is gone. A failure that happened **once**
    /// was lost exactly that way: the tree printed the failing test's name
    /// and the reader cut it.
    ///
    /// So a failure keeps its files, names them, and a success does not.
    #[test]
    fn a_failing_command_leaves_its_output_behind() {
        let out = command("sh", &["-c", "echo the evidence; exit 4"], Path::new(".")).expect("sh runs");
        assert!(!out.ok());
        assert!(!out.kept.is_empty(), "a failure kept nothing: {:?}", out.kept);
        for path in &out.kept {
            assert!(path.is_file(), "{} is not there", path.display());
        }
        let all = out.all();
        assert!(all.contains("the evidence"), "{all}");
        assert!(all.contains("kept"), "the report does not say the output was kept: {all}");

        // And a success keeps nothing: a gate that runs forty times a day
        // must not leave forty folders behind.
        let ok = command("sh", &["-c", "exit 0"], Path::new(".")).expect("sh runs");
        assert!(ok.ok());
        assert!(ok.kept.is_empty(), "a success kept files: {:?}", ok.kept);
    }

    #[test]
    fn a_silent_failure_still_says_something_useful() {
        let out = command("sh", &["-c", "exit 7"], Path::new(".")).expect("sh runs");
        let all = out.all();
        assert!(all.contains('7'), "an exit code with no output must still name itself: {all}");
    }

    #[test]
    fn a_child_that_writes_a_lot_is_still_captured() {
        // The whole reason for the file-based capture: a pipe-based one can
        // lose a megabyte of compiler output without saying so.
        let out = command("sh", &["-c", "yes craftkit | head -n 20000"], Path::new(".")).expect("sh runs");
        assert!(out.ok(), "{}", out.stderr);
        assert!(out.stdout.len() > 100_000, "captured {} bytes", out.stdout.len());
        assert!(out.stdout.matches("craftkit").count() >= 20_000, "output was truncated");
    }

    #[test]
    fn capture_files_are_removed_afterwards() {
        // The two paths this invocation creates, checked directly: a global
        // count would be wrong, because the tests run in parallel and other
        // captures are legitimately in flight.
        let capture = Capture::new("craftkit-capture-test").expect("creates");
        let paths = capture.paths.clone();
        for path in &paths {
            std::fs::write(path, "hello").expect("write");
        }
        assert_eq!(capture.read(&paths[0]).expect("reads").trim(), "hello");
        capture.cleanup().expect("cleans up");
        for path in &paths {
            assert!(!path.exists(), "{} was left behind", path.display());
        }
    }

    #[test]
    fn each_capture_gets_its_own_files() {
        let first = Capture::new("craftkit-capture-test").expect("creates");
        let second = Capture::new("craftkit-capture-test").expect("creates");
        assert_ne!(first.paths[0], second.paths[0], "two live captures must not share a file");
        let _ = first.cleanup();
        let _ = second.cleanup();
    }

    #[test]
    fn a_missing_program_is_an_error_not_a_panic() {
        let err = command("craftkit-no-such-program", &[], Path::new(".")).expect_err("must not find it");
        assert!(err.contains("could not run"), "{err}");
    }
}
