//! A caption file is **written**.
//!
//! The tree's own writers: the cues are numbered from one, the clock
//! is rounded to the nearest millisecond, and a blank line inside the
//! text of a cue is dropped.

use craftkit_sdk::serde_json::json;
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
fn a_file(name: &str) -> String {
    let path = craftkit_scratch::file(&format!("craftkit-v2-srtwrite-{name}-{}", std::process::id()))
        .expect("the scratch is made");
    path.to_string_lossy().to_string()
}

/// The **text** of a file that was written.
fn text_of(path: &str) -> String {
    let bytes = std::fs::read(path).expect("the file is there");
    String::from_utf8_lossy(&bytes).to_string()
}

/// A SubRip file is written: the cues are numbered from one, and the
/// clocks are as they are.
#[test]
fn a_subrip_file_is_written() {
    let mut session = session();
    let path = a_file("subrip");
    session
        .execute(
            "film.captions.write",
            &json!({
                "path": path,
                "cues": [
                    { "start": 1000, "end": 3500, "text": "First line" },
                    { "start": 4000, "end": 5000, "text": "Bye" },
                ],
            }),
        )
        .expect("the file is written");
    let written = text_of(&path);
    assert_eq!(
        written, "1\n00:00:01,000 --> 00:00:03,500\nFirst line\n\n2\n00:00:04,000 --> 00:00:05,000\nBye\n\n",
        "the file is as the tree writes it"
    );
}

/// A blank line inside the text of a cue is **dropped**: it would end
/// the cue.
#[test]
fn a_blank_line_inside_a_cue_is_dropped() {
    let mut session = session();
    let path = a_file("blank");
    session
        .execute(
            "film.captions.write",
            &json!({
                "path": path,
                "cues": [{ "start": 0, "end": 1000, "text": "one\n\ntwo" }],
            }),
        )
        .expect("the file is written");
    let written = text_of(&path);
    assert_eq!(written, "1\n00:00:00,000 --> 00:00:01,000\none\ntwo\n\n", "the blank line is gone");
}

/// A WebVTT file is written: the signature, the blocks, the
/// identifier, the settings and the voice.
#[test]
fn a_webvtt_file_is_written() {
    let mut session = session();
    let path = a_file("vtt");
    session
        .execute(
            "film.captions.write",
            &json!({
                "path": path,
                "format": "webvtt",
                "blocks": ["NOTE a comment"],
                "cues": [
                    {
                        "start": 1000,
                        "end": 3500,
                        "id": "intro",
                        "settings": "line:90%",
                        "speaker": "Ann",
                        "text": "Hello there",
                    },
                ],
            }),
        )
        .expect("the file is written");
    let written = text_of(&path);
    assert!(written.starts_with("WEBVTT\n\n"), "the signature is first: {written}");
    assert!(written.contains("NOTE a comment"), "the block is kept: {written}");
    assert!(
        written.contains("intro\n00:00:01.000 --> 00:00:03.500 line:90%\n<v Ann>Hello there\n"),
        "and the cue: {written}"
    );
}

/// An arrow in the text of a cue is written so that it is **not** an
/// arrow: the tree's own rule.
#[test]
fn an_arrow_in_the_text_is_written_as_it_is_not_one() {
    let mut session = session();
    let path = a_file("arrow");
    session
        .execute(
            "film.captions.write",
            &json!({
                "path": path,
                "format": "webvtt",
                "cues": [{ "start": 0, "end": 1000, "text": "a --> b" }],
            }),
        )
        .expect("the file is written");
    let written = text_of(&path);
    assert!(written.contains("a --&gt; b"), "the arrow is written: {written}");
}

/// What is written is read back: the cues are the cues.
#[test]
fn what_is_written_is_read_back() {
    let mut session = session();
    let path = a_file("round");
    session
        .execute(
            "film.captions.write",
            &json!({
                "path": path,
                "cues": [
                    { "start": 1000, "end": 3500, "text": "First line" },
                    { "start": 4000, "end": 5000, "text": "Bye" },
                ],
            }),
        )
        .expect("the file is written");
    let outcome = session.execute("film.captions.read", &json!({ "path": path })).expect("the cues are read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 2, "there are two cues: {outcome:?}");
    assert_eq!(cues[0]["start"], json!(1000), "the first: {outcome:?}");
    assert_eq!(cues[0]["text"], json!("First line"), "and its text: {outcome:?}");
    assert_eq!(cues[1]["end"], json!(5000), "the second: {outcome:?}");
}

/// A cue that **ends before it starts** is refused, and so is a cue
/// whose times are not whole milliseconds.
#[test]
fn a_cue_that_is_not_a_cue_is_refused() {
    let mut session = session();
    let path = a_file("bad");
    let outcome = session.execute(
        "film.captions.write",
        &json!({ "path": path, "cues": [{ "start": 5000, "end": 4000, "text": "x" }] }),
    );
    assert!(outcome.is_err(), "the cue is refused: {outcome:?}");
    let outcome = session.execute(
        "film.captions.write",
        &json!({ "path": path, "cues": [{ "start": 1.5, "end": 4000, "text": "x" }] }),
    );
    assert!(outcome.is_err(), "and so is a time that is not whole: {outcome:?}");
    let outcome =
        session.execute("film.captions.write", &json!({ "path": path, "format": "substation", "cues": [] }));
    assert!(outcome.is_err(), "and a format that is not one of them: {outcome:?}");
}
