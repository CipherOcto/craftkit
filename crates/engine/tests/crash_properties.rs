//! The crash report's promise, written as a property.
//!
//! A crash report is **input**: it arrives from somebody else's disk, and the
//! reader bounds what it will accept. The tree has three readers, and two of
//! them have property tests — the design file's
//! (`crates/io/tests/properties.rs`) and the commands'
//! (`crates/engine/tests/properties.rs`).
//!
//! The third had none, and it did not keep the promise the others keep:
//! **if a file was read, what it holds must be valid.** §183.
//!
//! # The corpus is the tree's own
//!
//! §180's rule is that a fuzzer's corpus is written by the tree's own
//! program, so the format being broken is the format the program speaks. Here
//! it costs nothing: the reader and the writer are in the same crate, so the
//! seed is `crash::build`'s own report and the tree's own writer writes it.
//!
//! # The reader is not behind the guard
//!
//! `Session::execute` runs every command under a `catch_unwind`, so "no
//! command panics" is a property that cannot fail (§175). **This reader is
//! not behind that guard** — `crash::show_in` is a plain function — so a
//! panic in it reaches the test and fails it. The property can fail here,
//! which is the only reason it is worth having.

use craftkit_engine::Session;
use craftkit_engine::crash;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use serde_json::{Value as Json, json};

/// The seed the suites run from.
///
/// **Fixed**, so a CI run can never fail at random, and the failures are not
/// written to a file: a file the tool writes is a list nobody reviews.
/// §175's rule. §177.
const SEED: u64 = 0x5eed_c0de;

/// How many cases a suite runs.
const CASES: u32 = 256;

/// The configuration every suite here runs with.
fn config() -> Config {
    Config { failure_persistence: None, rng_seed: RngSeed::Fixed(SEED), cases: CASES, ..Config::default() }
}

/// A report, written by the tree's own writer.
///
/// `crash::build` is the tree's own report builder — the same call the panic
/// hook makes — so the text this returns is the format the tree speaks.
fn a_report() -> crash::CrashReport {
    let mut session = Session::sample();
    let _ = session.execute("node.add", &json!({ "name": "Probe", "kind": "filter" }));
    crash::build(&session, "node.add", "the probe's own crash")
}

/// The report's text, as the tree writes it.
fn the_text(report: &crash::CrashReport) -> String {
    serde_json::to_string(report).expect("the tree's own report writes")
}

/// The folder the reader reads from.
///
/// One folder, one file, overwritten every time: the cases are scratch, and a
/// folder per case would be a thousand folders.
fn the_folder() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("craftkit-crash-properties").join(format!("{}", std::process::id()));
    std::fs::create_dir_all(dir.join("probe")).expect("the folder");
    dir
}

/// Write a report's text where the reader will find it.
fn write_report(text: &str) -> std::path::PathBuf {
    let dir = the_folder();
    std::fs::write(dir.join("probe").join("report.json"), text).expect("the report is written");
    dir
}

/// Every value in a tree, and where it is.
fn leaves(value: &Json, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    match value {
        Json::Object(map) => {
            for (index, (_, item)) in map.iter().enumerate() {
                path.push(index);
                leaves(item, path, out);
                path.pop();
            }
        }
        Json::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                path.push(index);
                leaves(item, path, out);
                path.pop();
            }
        }
        _ => out.push(path.clone()),
    }
}

/// Put a value at a place in a tree, if the place is there.
fn set_at(value: &mut Json, path: &[usize], new: Json) -> bool {
    let Some((first, rest)) = path.split_first() else {
        *value = new;
        return true;
    };
    match value {
        Json::Object(map) => {
            let Some(key) = map.keys().nth(*first).cloned() else { return false };
            let Some(item) = map.get_mut(&key) else { return false };
            set_at(item, rest, new)
        }
        Json::Array(items) => match items.get_mut(*first) {
            Some(item) => set_at(item, rest, new),
            None => false,
        },
        _ => false,
    }
}

/// The hostile values for one place in a report.
///
/// They are the values that reach the reader's own rules rather than being
/// turned away as "not JSON": a number past the model's limit, a range that is
/// empty, a note longer than the model allows, and the shapes that are not a
/// document at all.
fn a_hostile_value(index: usize) -> Json {
    match index % 8 {
        0 => json!(0),
        1 => json!(i64::MAX),
        2 => json!(-1),
        3 => json!(""),
        4 => json!("x".repeat(4096)),
        5 => json!(true),
        6 => json!(null),
        _ => json!([]),
    }
}

/// The promise: if a report was read, the documents it holds are ones the
/// model accepts.
///
/// This is the io reader's oracle, and it is the tree's promise about every
/// document that arrives from outside. §183.
fn the_promise(report: &crash::CrashReport) -> Result<(), String> {
    report.design.validate().map_err(|e| format!("the report's design is not usable: {e}"))?;
    report.base.validate().map_err(|e| format!("the report's base design is not usable: {e}"))?;
    Ok(())
}

proptest! {
    #![proptest_config(config())]

    /// A report with one byte changed is read or refused.
    ///
    /// The byte is changed at a *generated* position, so the case that matters
    /// — the one that breaks something — is not left to luck. §175's lesson.
    #[test]
    fn a_report_with_one_byte_changed_is_read_or_refused(
        at in 0usize..8192,
        byte in any::<u8>(),
    ) {
        let mut bytes = the_text(&a_report()).into_bytes();
        let at = at % bytes.len().max(1);
        if at < bytes.len() {
            bytes[at] = byte;
        }
        let text = String::from_utf8_lossy(&bytes).to_string();
        let dir = write_report(&text);
        if let Ok(report) = crash::show_in(&dir, "probe") {
            prop_assert!(the_promise(&report).is_ok(), "a report that was read holds a document the model refuses");
        }
    }

    /// A report cut short is read or refused.
    ///
    /// A truncated report is the commonest damaged report there is: the
    /// process died, the disk filled, the copy was interrupted.
    #[test]
    fn a_truncated_report_is_read_or_refused(at in 0usize..8192) {
        let text = the_text(&a_report());
        let cut = at % text.len().max(1);
        let shortened = &text[..cut];
        let dir = write_report(shortened);
        if let Ok(report) = crash::show_in(&dir, "probe") {
            prop_assert!(the_promise(&report).is_ok(), "a report that was read holds a document the model refuses");
        }
    }

    /// A report with one hostile value in it is read or refused.
    ///
    /// The value goes where a value belongs, so it reaches the reader's own
    /// rules rather than being turned away as "not JSON".
    #[test]
    fn a_report_with_one_hostile_value_is_read_or_refused(
        leaf in 0usize..256,
        hostile in 0usize..8,
    ) {
        let Ok(mut tree) = serde_json::from_str::<Json>(&the_text(&a_report())) else {
            return Err(TestCaseError::reject("the seed report is not json"));
        };
        let mut paths = Vec::new();
        leaves(&tree, &mut Vec::new(), &mut paths);
        if paths.is_empty() {
            return Err(TestCaseError::reject("the seed report has no values"));
        }
        let path = paths[leaf % paths.len()].clone();
        prop_assert!(set_at(&mut tree, &path, a_hostile_value(hostile)), "the path is not there");
        let text = serde_json::to_string(&tree).expect("the tree is written");
        let dir = write_report(&text);
        if let Ok(report) = crash::show_in(&dir, "probe") {
            prop_assert!(the_promise(&report).is_ok(), "a report that was read holds a document the model refuses");
        }
    }

    /// The replay door refuses a report or replays it, and never panics.
    ///
    /// The steps in a report are **commands**, so this is the door where a
    /// hostile report aims instructions at the engine — §177's subject with a
    /// different input. The reader is not behind the engine's guard, so a
    /// panic in it reaches this test and fails it.
    ///
    /// And what comes back is about documents the model accepts: the report's
    /// design was checked by the reader, and the document the replay built
    /// starts from a checked base and is changed only by commands that keep
    /// it valid.
    #[test]
    fn the_replay_door_refuses_or_replays(
        leaf in 0usize..256,
        hostile in 0usize..8,
    ) {
        let Ok(mut tree) = serde_json::from_str::<Json>(&the_text(&a_report())) else {
            return Err(TestCaseError::reject("the seed report is not json"));
        };
        let mut paths = Vec::new();
        leaves(&tree, &mut Vec::new(), &mut paths);
        if paths.is_empty() {
            return Err(TestCaseError::reject("the seed report has no values"));
        }
        let path = paths[leaf % paths.len()].clone();
        prop_assert!(set_at(&mut tree, &path, a_hostile_value(hostile)), "the path is not there");
        let text = serde_json::to_string(&tree).expect("the tree is written");
        let dir = write_report(&text);
        if let Ok(outcome) = crash::reproduce_in(&dir, "probe") {
            prop_assert_eq!(
                outcome.results.len(),
                outcome.steps,
                "the replay says it ran {} step(s) and reports {} result(s)",
                outcome.steps,
                outcome.results.len()
            );
            prop_assert!(
                outcome.failures.len() <= outcome.steps,
                "the replay reports {} failure(s) for {} step(s)",
                outcome.failures.len(),
                outcome.steps
            );
        }
    }
    /// One folder, one answer.
    ///
    /// The tree's doors must agree about a report. The folder used to hold a
    /// copy of the steps beside the report, and `replay_in` read that copy
    /// while `show_in` read the report — so a folder whose two copies
    /// disagreed had two answers, and the tree noticed neither. §185.
    ///
    /// The second copy is written here, saying something else, so the case
    /// that matters is not left to luck: a report arrives from another
    /// machine, and it may carry anything.
    #[test]
    fn a_folder_with_a_second_copy_of_the_steps_has_one_answer(
        commands in proptest::collection::vec("[a-z]{1,4}", 0..6),
    ) {
        let report = a_report();
        let dir = the_folder();
        // Its own folder: the cases above share one scratch report, and they
        // run at the same time.
        let folder = dir.join("one-answer");
        std::fs::create_dir_all(&folder).expect("the folder");
        // The report, written by the tree's own writer.
        std::fs::write(folder.join("report.json"), the_text(&report)).expect("the report is written");
        // And a second copy of the steps, saying something else.
        let other: Vec<Json> =
            commands.iter().map(|name| json!({ "command": name, "params": {} })).collect();
        std::fs::write(
            folder.join("replay.json"),
            serde_json::to_string(&other).expect("the copy is written"),
        )
        .expect("the copy is written");

        let shown = crash::show_in(&dir, "one-answer").expect("the report reads");
        let replayed = crash::replay_in(&dir, "one-answer").expect("the steps read");
        prop_assert_eq!(
            replayed.len(),
            shown.replay.len(),
            "the doors disagree about how many commands the report holds"
        );
        prop_assert_eq!(replayed, shown.replay, "the doors disagree about which commands");
    }
}

/// The report's own shape, as the tree writes it.
///
/// A test, not a property: the corpus has to be a report the tree wrote, or
/// every case above is a case about a file nothing else in the tree speaks.
#[test]
fn the_corpus_is_a_report_the_tree_wrote() {
    let report = a_report();
    let text = the_text(&report);
    let tree: Json = serde_json::from_str(&text).expect("the tree's report is json");
    assert_eq!(tree["command"], "node.add");
    assert!(tree["design"].is_object(), "the report has no design");
    assert!(tree["base"].is_object(), "the report has no base");
    assert!(tree["replay"].is_array(), "the report has no replay");
    assert!(!report.replay.is_empty(), "the report has no steps");
}
