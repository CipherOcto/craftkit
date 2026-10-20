//! The crash pipeline, end to end: a report is recorded, then read back by the
//! real binary.
//!
//! `craftkit-engine`'s own tests round-trip a report through a temporary folder
//! and the panic hook in `apps/craftkit` is tested where it fires. What nothing
//! covered was the half a person actually uses: `craftkit-cli crashes`, which
//! reads the *real* crash folder. On a machine where nothing has ever crashed,
//! that folder is empty — so the command an agent is told to run first after
//! "the app crashed" had never run against a real report.
//!
//! `CRAFTKIT_CRASH_DIR` relocates the folder, so this runs in CI and writes
//! nothing into anybody's home.

use std::path::{Path, PathBuf};
use std::process::Command;

use craftkit_engine::Session;

const BIN: &str = env!("CARGO_BIN_EXE_craftkit-cli");

/// A temporary crash folder, removed when the test ends.
struct CrashDir(PathBuf);

impl CrashDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("craftkit-crash-cli-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a writable folder");
        CrashDir(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for CrashDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Run `craftkit-cli crashes …` against this folder.
fn crashes(dir: &CrashDir, args: &[&str]) -> (bool, String) {
    let out = Command::new(BIN)
        .arg("crashes")
        .args(args)
        .env(craftkit_engine::crash::CRASH_DIR_ENV, dir.path())
        .output()
        .expect("the binary that cargo just built");
    (out.status.success(), String::from_utf8_lossy(&out.stdout).to_string())
}

/// A session that has done something worth replaying.
fn busy_session() -> Session {
    let mut s = Session::sample();
    s.execute("node.add", &serde_json::json!({ "name": "Before the crash", "kind": "filter" }))
        .expect("a valid command");
    s
}

#[test]
fn a_recorded_report_is_listed_shown_and_replayed_by_the_real_binary() {
    let dir = CrashDir::new("pipeline");
    let session = busy_session();
    let report = craftkit_engine::crash::build(&session, "key.set", "something went wrong");
    let id = report.id.clone();
    craftkit_engine::crash::write_to(dir.path(), &report).expect("the report is written");

    let (ok, text) = crashes(&dir, &["list"]);
    assert!(ok, "list failed: {text}");
    assert!(text.contains(&id), "`list` did not name the report: {text:?}");

    let (ok, text) = crashes(&dir, &["show", &id]);
    assert!(ok, "show failed: {text}");
    let shown: serde_json::Value = serde_json::from_str(&text).expect("show prints one JSON document");
    assert_eq!(shown["command"], "key.set", "{shown}");
    assert_eq!(shown["message"], "something went wrong", "{shown}");

    // The property that makes a crash report worth writing: the design and the
    // steps come back out of the folder and run again — onto the design the
    // report recorded — and the CLI says whether the result matched.
    let (ok, text) = crashes(&dir, &["replay", &id]);
    assert!(ok, "replay failed: {text}");
    assert!(text.contains("Before the crash"), "the replay lost the document: {text:?}");
    assert!(text.contains("reproduced the design in the report"), "{text:?}");
}

/// A report whose recorded design does not match what its commands build.
///
/// The CLI used to print "N step(s) replayed" and stop, so a replay that
/// produced a different document looked exactly like one that did not. §47.
#[test]
fn a_replay_that_does_not_match_the_report_says_so() {
    let dir = CrashDir::new("diverged");
    let session = busy_session();
    let mut report = craftkit_engine::crash::build(&session, "key.set", "something went wrong");
    // The report now claims a document the recorded commands do not build.
    let mut other = craftkit_model::Document::empty("lies");
    other.add_node("Something else", craftkit_model::NodeKind::Source).expect("a node");
    report.design = other;
    let id = report.id.clone();
    craftkit_engine::crash::write_to(dir.path(), &report).expect("the report is written");

    let (ok, text) = crashes(&dir, &["replay", &id]);
    assert!(ok, "replay failed: {text}");
    assert!(text.contains("DIVERGED"), "a replay that did not match must say so: {text:?}");
    assert!(text.contains("Something else"), "the difference must be named: {text:?}");
}

/// The replay as a script: the commands *and* the document they start from.
///
/// A script without the base would rebuild the commands on an empty document
/// — a different design from the one that crashed, for any report whose
/// session had loaded a file. §47.
#[test]
fn the_replay_can_be_printed_as_a_script_that_carries_its_base() {
    let dir = CrashDir::new("script");
    let session = busy_session();
    let report = craftkit_engine::crash::build(&session, "key.set", "something went wrong");
    let id = report.id.clone();
    craftkit_engine::crash::write_to(dir.path(), &report).expect("the report is written");

    let (ok, text) = crashes(&dir, &["replay", &id, "--script"]);
    assert!(ok, "replay --script failed: {text}");
    let script: serde_json::Value = serde_json::from_str(&text).expect("the script is one JSON document");
    let commands = script["commands"].as_array().expect("the commands are an array");
    assert_eq!(commands.len(), 1, "{script}");
    assert_eq!(commands[0]["command"], "node.add", "{script}");
    assert_eq!(commands[0]["params"]["name"], "Before the crash", "{script}");
    assert!(script["base"]["nodes"].is_array(), "the script carries the document it starts from: {script}");
}

#[test]
fn an_empty_folder_says_so_rather_than_printing_nothing() {
    let dir = CrashDir::new("empty");
    let (ok, text) = crashes(&dir, &["list"]);
    assert!(ok, "list failed: {text}");
    assert!(text.contains("no crash reports"), "{text:?}");
    // Silence would be indistinguishable from a folder that could not be read.
}

#[test]
fn an_unknown_report_is_refused_by_name_rather_than_invented() {
    let dir = CrashDir::new("unknown");
    let (ok, _) = crashes(&dir, &["show", "20250101-000000-dead"]);
    assert!(!ok, "showing a report that does not exist must fail");

    let (ok, _) = crashes(&dir, &["replay", "20250101-000000-dead"]);
    assert!(!ok, "replaying a report that does not exist must fail");
}
