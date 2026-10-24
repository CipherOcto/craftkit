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

/// A report that arrives from another machine has a door.
///
/// The report is **input**: the recipe asks for `report.json` from whoever
/// had the crash, so the file arrives with no crash folder around it and no
/// id to look it up by. Every door in the tree read a report by id, out of a
/// folder, so an arriving report had **no door at all** — `run`, `eval` and
/// `snapshot` refused it, and the only way to read one was to copy it into
/// the crash folder under its own id, by hand. §195.
///
/// So this test points the crash folder at an **empty** folder and gives the
/// CLI the file. The file is the only copy there is, and the crash folder is
/// not holding it.
#[test]
fn a_report_that_arrives_in_a_file_is_replayed_by_the_real_binary() {
    let incoming = CrashDir::new("incoming-file");
    let empty = CrashDir::new("empty-crash-folder");
    let session = busy_session();
    let report = craftkit_engine::crash::build(&session, "key.set", "something went wrong");

    // The report in a file, in a folder of its own that is not the crash
    // folder: the shape of a report that arrived from somewhere else.
    let path = incoming.path().join("report.json");
    std::fs::write(&path, serde_json::to_string(&report).expect("the report prints"))
        .expect("the file is written");

    let (ok, text) = crashes(&empty, &["replay", "--file", path.to_str().expect("a path")]);
    assert!(ok, "the replay of an arriving report failed: {text}");
    assert!(text.contains("Before the crash"), "the replay lost the document: {text:?}");
    assert!(text.contains("reproduced the design in the report"), "{text:?}");

    // And it can be read, without the folder and without the id.
    let (ok, text) = crashes(&empty, &["show", "--file", path.to_str().expect("a path")]);
    assert!(ok, "showing an arriving report failed: {text}");
    let shown: serde_json::Value = serde_json::from_str(&text).expect("show prints one JSON document");
    assert_eq!(shown["message"], "something went wrong", "{shown}");
}

/// A file that is not a report is refused, and the reason names the file.
///
/// The file door is a door for untrusted input: the path comes from a person
/// and may point at anything. §195.
#[test]
fn a_file_that_is_not_a_report_is_refused_by_the_real_binary() {
    let incoming = CrashDir::new("not-a-report");
    let empty = CrashDir::new("empty-crash-folder-2");
    let path = incoming.path().join("something-else.json");
    std::fs::write(&path, "{\"hello\": true}").expect("the file is written");

    let (ok, _) = crashes(&empty, &["show", "--file", path.to_str().expect("a path")]);
    assert!(!ok, "a file that is not a report must be refused");
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
