//! The reader's promise, written as a property.
//!
//! The tree's promise is that a hostile file is **refused**, not that it
//! crashes. The hand-written tests check the files somebody thought of —
//! twenty of them, each one a case somebody imagined. This file checks the
//! rest: the files nobody imagined.
//!
//! §177.
//!
//! # What is measured, and what is not
//!
//! The property is that the reader **answers**. It may answer with a file or
//! with a refusal; what it may not do is panic, and it may not answer with a
//! file that holds a document the model refuses.
//!
//! The second half is not circular: the reader calls the model's own
//! `validate`, and the property calls it again on the result. What the
//! property adds is the *order* — a file that is read must hold a document
//! that passes — and it catches a reader that stopped checking.

use craftkit_io::format::File;
use craftkit_model::{Document, NodeKind};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use serde_json::{Value as Json, json};

/// The seed the suites run from.
///
/// **Fixed**, so a CI run can never fail at random, and the failures are not
/// written to a file: a file the tool writes is a list nobody reviews. The
/// family's practice, and §175's rule. §177.
const SEED: u64 = 0x5eed_9a7f;

/// How many cases a suite runs.
const CASES: u32 = 256;

/// The configuration every suite here runs with.
fn config() -> Config {
    Config { failure_persistence: None, rng_seed: RngSeed::Fixed(SEED), cases: CASES, ..Config::default() }
}

/// A file this build wrote, with nodes in it, to damage.
fn a_valid_file() -> String {
    let mut doc = Document::empty("Probe");
    for (name, kind) in [("Level", NodeKind::Source), ("Taper", NodeKind::Filter), ("Out", NodeKind::Sink)] {
        let id = doc.add_node(name, kind).expect("the node is added");
        if let Some(node) = doc.get_mut(id) {
            let with_defaults = node.clone().with_defaults(true, 1.0).expect("the defaults are set");
            *node = with_defaults;
        }
    }
    File::new(doc).to_text().expect("the file is written")
}

/// The places a value can sit in a file, in a stable order.
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

/// Put a value at a path, if the path is there.
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

/// The values a hostile file is made of.
///
/// They are the ones the tree's own rules are about: a number too big, a
/// string too long, a null where a value belongs, a list where an object
/// belongs.
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

proptest! {
    #![proptest_config(config())]

    /// A file with one byte changed is read or refused.
    ///
    /// The byte is changed at a *generated* position, so the case that matters
    /// — the one that breaks something — is not left to luck. §175's lesson.
    #[test]
    fn a_file_with_one_byte_changed_is_read_or_refused(
        at in 0usize..4096,
        byte in any::<u8>(),
    ) {
        let mut bytes = a_valid_file().into_bytes();
        let at = at % bytes.len().max(1);
        if at < bytes.len() {
            bytes[at] = byte;
        }
        let text = String::from_utf8_lossy(&bytes).to_string();
        if let Ok(file) = File::from_text(&text) {
            prop_assert!(
                file.document.validate().is_ok(),
                "a file that was read holds a document the model refuses"
            );
        }
    }

    /// A file cut short is read or refused.
    ///
    /// A truncated file is the commonest damaged file there is: the process
    /// died, the disk filled, the copy was interrupted.
    #[test]
    fn a_truncated_file_is_read_or_refused(at in 0usize..4096) {
        let text = a_valid_file();
        let cut = at % text.len().max(1);
        let shortened = &text[..cut];
        if let Ok(file) = File::from_text(shortened) {
            prop_assert!(
                file.document.validate().is_ok(),
                "a file that was read holds a document the model refuses"
            );
        }
    }

    /// A file with one hostile value in it is read or refused.
    ///
    /// The value goes where a value belongs, so it reaches the reader's own
    /// rules rather than being turned away as "not JSON".
    #[test]
    fn a_file_with_one_hostile_value_is_read_or_refused(
        leaf in 0usize..256,
        hostile in 0usize..8,
    ) {
        let Ok(mut tree) = serde_json::from_str::<Json>(&a_valid_file()) else {
            return Err(TestCaseError::reject("the seed file is not json"));
        };
        let mut paths = Vec::new();
        leaves(&tree, &mut Vec::new(), &mut paths);
        if paths.is_empty() {
            return Err(TestCaseError::reject("the seed file has no values"));
        }
        let path = paths[leaf % paths.len()].clone();
        prop_assert!(set_at(&mut tree, &path, a_hostile_value(hostile)), "the path is not there");
        let text = serde_json::to_string(&tree).expect("the tree is written");
        if let Ok(file) = File::from_text(&text) {
            prop_assert!(
                file.document.validate().is_ok(),
                "a file that was read holds a document the model refuses"
            );
        }
    }
}
