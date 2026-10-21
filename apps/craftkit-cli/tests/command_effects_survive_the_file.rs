//! Every command's effect has to survive the file.
//!
//! Two rules meet here that are documented separately and never tested together.
//! *Everything is a command*: user-visible behaviour is an entry in the registry
//! and nothing else. And `.craftkit` is the only persistence there is. So a
//! command that changes the document in a way the format does not carry is
//! work that vanishes when the file is closed — and it vanishes **quietly**,
//! because the command reported success and the file is a valid plan.
//!
//! Nothing checked this. The round-trip tests that existed were at the model
//! level (`documents_round_trip_through_json_and_keep_the_id_counter`), which
//! proves the *type* survives, not that the *commands'* effects do. A new field
//! that only one command writes would pass every test in the tree.
//!
//! It lives here rather than in `craftkit-io` because it needs both crates, and
//! `craftkit-io` is a *lower* layer than `craftkit-engine`: a dev-dependency
//! upwards would be a layering violation that `cargo xtask layers` correctly
//! reports. This is an application-level statement — "the file the application
//! writes is the file it read" — so an application-level test is the right home.

use craftkit_engine::Session;
use serde_json::{Value as Json, json};

/// One command under test: what to run first, what to run, and with what.
///
/// The setup exists because half the registry refuses to act on a document that
/// has nothing to act on — `key.ease` and `key.remove` need a key at that
/// instant, and `prop.reset` needs a key to remove. A refusal is correct
/// behaviour and would make the test fail for the wrong reason.
struct Case {
    setup: Vec<(&'static str, Json)>,
    command: &'static str,
    params: Json,
}

fn case(command: &'static str, params: Json) -> Case {
    Case { setup: Vec::new(), command, params }
}

/// A case that needs a keyframe to exist before the command under test runs.
fn after_key(command: &'static str, params: Json) -> Case {
    Case {
        setup: vec![("key.set", json!({ "node": 1, "path": "$level", "value": "0.5", "time": 48 }))],
        command,
        params,
    }
}

/// Every command that changes the document.
///
/// One entry per mutating command in the registry. `the_registry_and_this_table_agree`
/// is what keeps it complete.
fn cases() -> Vec<Case> {
    vec![
        case("doc.note", json!({ "text": "A note about the design." })),
        case("doc.rename", json!({ "name": "Renamed plan" })),
        case("doc.range", json!({ "start": 48, "end": 96 })),
        case("node.add", json!({ "name": "Extra", "kind": "filter" })),
        case("node.duplicate", json!({ "node": 1 })),
        case("node.rename", json!({ "node": 2, "name": "Second" })),
        case("node.kind", json!({ "node": 3, "kind": "filter" })),
        case("node.move", json!({ "node": 3, "index": 0 })),
        case("node.mute", json!({ "node": 2, "muted": true })),
        case("node.remove", json!({ "node": 3 })),
        case("prop.set", json!({ "node": 1, "path": "$level", "value": "0.25" })),
        after_key("prop.reset", json!({ "node": 1, "path": "$level" })),
        after_key("key.set", json!({ "node": 1, "path": "$level", "value": "0.75", "time": 24 })),
        after_key("key.ease", json!({ "node": 1, "path": "$level", "easing": "hold", "time": 48 })),
        after_key("key.remove", json!({ "node": 1, "path": "$level", "time": 48 })),
        after_key("key.clear", json!({ "node": 1, "path": "$level" })),
    ]
}

/// The commands that are not in the table, and why.
///
/// The second half is the interesting one. Three of these are session state and
/// are *deliberately* absent from the file; four read or report and change
/// nothing at all. Listing them with a reason is what makes a new command a
/// decision rather than an oversight: `the_registry_and_this_table_agree` fails
/// on any id that appears in neither list.
const NOT_IN_THE_FILE: &[(&str, &str)] = &[
    ("playback.seek", "the playhead is session state; a file records the range, not where you were"),
    ("history.undo", "undo history is session state and is never written to a file"),
    ("history.redo", "undo history is session state and is never written to a file"),
    ("diagnostics.list", "reads the plan and changes nothing"),
    ("plan.evaluate", "computes the plan and changes nothing"),
    ("session.stats", "reports counters and changes nothing"),
];

/// A document that went through `save_text` and `load_text`, as JSON.
fn round_tripped(session: &Session) -> Json {
    let text = craftkit_io::save_text(session.document()).expect("the document saves");
    let back = craftkit_io::load_text(&text).expect("what was written reads back");
    serde_json::to_value(&back).expect("the document is json")
}

/// **Every command's effect survives the file.**
///
/// The first half is the part that is easy to skip and is what makes the second
/// half mean anything: the command has to *change* the document. A command that
/// silently did nothing would pass a round-trip assertion while proving nothing
/// — which is exactly what happened when this table was first written, where
/// `playback.seek` reported success for a playhead the document never had.
#[test]
fn every_commands_effect_survives_the_file() {
    for entry in cases() {
        let mut session = Session::sample();
        for (setup, params) in entry.setup {
            session.execute(setup, &params).unwrap_or_else(|e| panic!("setup {setup}: {e}"));
        }

        let before = serde_json::to_value(session.document()).expect("json");
        session
            .execute(entry.command, &entry.params)
            .unwrap_or_else(|e| panic!("{} refused: {e}", entry.command));
        let after = serde_json::to_value(session.document()).expect("json");

        assert_ne!(before, after, "{} changed nothing, so its round trip would prove nothing", entry.command);
        assert_eq!(
            round_tripped(&session),
            after,
            "{}'s effect did not survive save and load",
            entry.command
        );
    }
}

/// Two paths that share a name at different depths are two different paths.
///
/// The engine can create both — `a/trigger/lead` and then `trigger/lead` —
/// and the file has to carry them, because they are not competing for the
/// same name space. The reader used to call such a plan invalid, so the app
/// could save a file it could not open. §50.
#[test]
fn a_group_name_at_two_depths_survives_the_file() {
    let mut session = Session::sample();
    session.execute("prop.set", &json!({ "node": 1, "path": "a/trigger/lead", "value": 1 })).expect("ok");
    session.execute("prop.set", &json!({ "node": 1, "path": "trigger/lead", "value": 2 })).expect("ok");

    let after = serde_json::to_value(session.document()).expect("json");
    assert_eq!(round_tripped(&session), after, "the two groups did not survive the file");

    // And both are still there, under their own paths.
    let back = craftkit_io::load_text(&craftkit_io::save_text(session.document()).expect("saves"))
        .expect("reads back");
    let props = back.nodes[0].props.clone();
    assert!(
        props.find(&craftkit_model::PropPath::parse("trigger/lead").expect("path")).is_some(),
        "the top-level group is missing: {props:?}"
    );
    assert!(
        props.find(&craftkit_model::PropPath::parse("a/trigger/lead").expect("path")).is_some(),
        "the nested group is missing: {props:?}"
    );
}

/// Every command is accounted for, on one side or the other.
///
/// Without this the table above is a list of what somebody remembered, and the
/// next command added to the registry is silently unpersisted — the failure this
/// file exists to catch, wearing the face of a passing suite.
#[test]
fn the_registry_and_this_table_agree() {
    let catalogue = craftkit_engine::catalogue();
    let ids: Vec<&str> = catalogue
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|spec| spec["id"].as_str())
        .collect();
    assert!(ids.len() >= 20, "the registry is smaller than this file expects: {ids:?}");

    for id in &ids {
        let in_table = cases().iter().any(|c| c.command == *id);
        let listed = NOT_IN_THE_FILE.iter().any(|(n, _)| n == id);
        assert!(
            in_table || listed,
            "`{id}` is in neither CASES nor NOT_IN_THE_FILE: decide whether its effect is in \
             the file, and say which"
        );
    }

    // And nothing in either list is a command that has since been renamed away,
    // which would let the table drift out of date silently.
    for entry in cases() {
        assert!(ids.contains(&entry.command), "`{}` is in the table but not in the registry", entry.command);
    }
    for (id, reason) in NOT_IN_THE_FILE {
        assert!(ids.contains(id), "`{id}` is in NOT_IN_THE_FILE but not in the registry");
        assert!(reason.len() > 20, "`{id}` has no real reason recorded");
    }
}

/// The format document's example file is what the document says it is.
///
/// "That is a complete, loadable file. `craftkit-cli eval` on it reports one
/// node, 12 frames at 24 fps, and no diagnostics." — three claims about a
/// hand-written file, in the one document that describes the format, and
/// nothing checked them. A change to the format, or to the value rules
/// (§73), could make the example unloadable and the document would go on
/// saying it loads. §79.
#[test]
fn the_formats_example_file_is_what_the_document_says() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let doc = std::fs::read_to_string(root.join("docs/design/file-format.md"))
        .expect("the format document is in the repository");

    // The first json block is the example file.
    let after = doc.split("```json").nth(1).expect("the document has a json block");
    let text = after.split("```").next().expect("the block is closed").trim();

    // It loads.
    let document = craftkit_io::load_text(text).expect("the example file loads");

    // One node.
    assert_eq!(document.nodes.len(), 1, "the document says one node");

    // Twelve frames at 24 fps.
    let frames = document.meta.duration().frames_i64(document.meta.frame_rate());
    assert_eq!(frames, 12, "the document says 12 frames at 24 fps");
    assert_eq!(document.meta.fps, 24, "the document says 24 fps");

    // And no diagnostics.
    let mut session = Session::new("the example");
    session.load_document(document);
    session.execute("plan.evaluate", &json!({})).expect("the plan evaluates");
    let diagnostics = session.execute("diagnostics.list", &json!({})).expect("the diagnostics list");
    let items = diagnostics.data["items"].as_array().map(|items| items.len());
    assert_eq!(items, Some(0), "the document says no diagnostics: {}", diagnostics.data);
}

/// The two things a file does not carry are the two it is documented not to
/// carry.
///
/// This is a test about prose, which is unusual, and it is here because the
/// format's "What is *not* in the file" section lists the plan and the id
/// counter and says nothing about the playhead or the undo history — the two
/// absences a person actually notices, and the reason a reopened design starts
/// at frame zero with nothing to undo.
#[test]
fn the_format_says_which_session_state_it_does_not_carry() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let doc = std::fs::read_to_string(root.join("docs/design/file-format.md"))
        .expect("the format document is in the repository");

    let not_in = doc.split("## What is *not* in the file").nth(1).expect("the section exists");
    let section = not_in.split("\n## ").next().unwrap_or(not_in);
    for word in ["playhead", "undo"] {
        assert!(
            section.to_lowercase().contains(word),
            "the format's 'what is not in the file' section never mentions the {word}, and both \
             are left out of every saved file"
        );
    }
}
