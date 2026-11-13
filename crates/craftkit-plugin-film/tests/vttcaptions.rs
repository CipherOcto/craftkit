//! The **cues** of a **WebVTT** file.
//!
//! The reader keeps what the file says about its cues: the
//! identifier, the settings and the voice.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the film battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A file of this test's own, so that two tests do not fight over one
/// file.
fn a_file(name: &str, text: &str) -> String {
    let path = craftkit_scratch::file(&format!("craftkit-v2-vtt-{name}-{}", std::process::id()))
        .expect("the scratch is made");
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// The **cues** of a file, as the answer holds them.
fn cues_of(session: &mut Session, path: &str) -> Vec<Json> {
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "webvtt" }))
        .expect("the cues are read");
    outcome.data["cues"].as_array().expect("the cues are a list").clone()
}

/// A WebVTT file is read into its **cues**, and the file's own words
/// about them are kept: the identifier, the settings and the voice.
#[test]
fn a_webvtt_file_is_read_into_its_cues() {
    let mut session = session();
    let path = a_file(
        "sample",
        "WEBVTT - with header\nKind: captions\n\nintro\n00:01.000 --> 00:03.500 line:90% align:start\n<v.loud Ann Smith>Hello there\nsecond line\n\n00:00:04.000 --> 00:00:05.000\n<i>plain</i> &amp; tags\n",
    );
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 2, "there are two cues: {cues:?}");
    assert_eq!(cues[0]["id"], Json::from("intro"), "the identifier is kept: {cues:?}");
    assert_eq!(cues[0]["start"], Json::from(1000), "the start: {cues:?}");
    assert_eq!(cues[0]["end"], Json::from(3500), "the end: {cues:?}");
    assert_eq!(cues[0]["startClock"], Json::from("00:00:01.000"), "and the clock: {cues:?}");
    assert_eq!(cues[0]["settings"], Json::from("line:90% align:start"), "the settings are kept: {cues:?}");
    assert_eq!(cues[0]["speaker"], Json::from("Ann Smith"), "the voice is taken off: {cues:?}");
    assert_eq!(cues[0]["text"], Json::from("Hello there\nsecond line"), "and the text stays: {cues:?}");
    assert_eq!(cues[1]["id"], Json::Null, "the second cue has no identifier: {cues:?}");
    assert_eq!(cues[1]["text"], Json::from("<i>plain</i> &amp; tags"), "and its text: {cues:?}");
}

/// The **NOTE** blocks and the **STYLE** and **REGION** blocks before
/// the first cue are kept, as the tree keeps them.
#[test]
fn the_note_and_style_blocks_are_kept() {
    let mut session = session();
    let path = a_file(
        "blocks",
        "WEBVTT\n\nSTYLE\n::cue { color: yellow }\n\nNOTE a comment\nspanning lines\n\n00:00:01.000 --> 00:00:02.000\nHi\n",
    );
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "webvtt" }))
        .expect("the cues are read");
    let blocks = outcome.data["blocks"].as_array().expect("the blocks are a list");
    assert_eq!(blocks.len(), 2, "the two blocks are kept: {outcome:?}");
    assert_eq!(blocks[0], Json::from("STYLE\n::cue { color: yellow }"), "the style: {outcome:?}");
    assert_eq!(blocks[1], Json::from("NOTE a comment\nspanning lines"), "and the note: {outcome:?}");
}

/// A file with **no signature** is read anyway, and a file with no cue
/// and no signature is not a WebVTT file.
#[test]
fn a_file_with_no_signature_is_read_and_a_file_with_no_cue_is_refused() {
    let mut session = session();
    let path = a_file("nosig", "00:00:01.000 --> 00:00:02.000\nHi\n");
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "the cue is read: {cues:?}");

    let path = a_file("nocue", "this is not a WebVTT file\n");
    let outcome = session.execute("film.captions.read", &json!({ "path": path, "format": "webvtt" }));
    assert!(outcome.is_err(), "the file is refused: {outcome:?}");
}

/// A cue whose **end is not after its start** is skipped, and the
/// answer says so.
#[test]
fn a_cue_that_ends_before_it_starts_is_skipped() {
    let mut session = session();
    let path = a_file(
        "inverted",
        "WEBVTT\n\n00:00:05.000 --> 00:00:04.000\nX\n\n00:00:06.000 --> 00:00:07.000\nY\n",
    );
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "webvtt" }))
        .expect("the cues are read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 1, "the first cue is skipped: {outcome:?}");
    let warnings = outcome.data["warnings"].as_array().expect("the warnings are a list");
    assert_eq!(warnings.len(), 1, "and it is said: {outcome:?}");
}

/// A format that is **not one of the tree's** is refused by name.
#[test]
fn a_format_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let path = a_file("format", "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nHi\n");
    let outcome = session.execute("film.captions.read", &json!({ "path": path, "format": "substation" }));
    assert!(outcome.is_err(), "the format is refused: {outcome:?}");
}
