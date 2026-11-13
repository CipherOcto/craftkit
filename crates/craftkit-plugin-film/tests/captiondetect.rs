//! The **format** of a caption file is detected.
//!
//! The extension of the file and the content of it say together what
//! the file is, and a file they do not name is not a caption file. The
//! rules are the tree's own.

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
fn a_file(name: &str, text: &str) -> String {
    let directory = craftkit_scratch::dir(&format!("craftkit-v2-captiondetect-{}", std::process::id()))
        .expect("the directory is made");
    let path = directory.join(name);
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// The **format** of a file, as the answer holds it.
fn format_of(session: &mut Session, path: &str) -> String {
    let outcome =
        session.execute("film.captions.detect", &json!({ "path": path })).expect("the file is looked at");
    outcome.data["format"].as_str().expect("the format is named").to_string()
}

/// A **SubRip file** is detected: the extension says it and the head
/// holds a timing line.
#[test]
fn a_subrip_file_is_detected() {
    let mut session = session();
    let path = a_file("the-subrip.srt", "1\n00:00:01,000 --> 00:00:02,000\nHello\n");
    assert_eq!(format_of(&mut session, &path), "subrip", "the file is a SubRip file");
}

/// An **SCC file** is detected: the head of it says what it is.
#[test]
fn an_scc_file_is_detected() {
    let mut session = session();
    let path = a_file(
        "the-scc.scc",
        "Scenarist_SCC V1.0\n\n00:00:00;20\t9420 9420 94ae 94ae 94d0 94d0 c845 4c4c 4f80 942f 942f\n",
    );
    assert_eq!(format_of(&mut session, &path), "scc", "the file is an SCC file");
}

/// An **MCC file** is detected: the head of it says what it is.
#[test]
fn an_mcc_file_is_detected() {
    let mut session = session();
    let path = a_file("the-mcc.mcc", "File Format=MacCaption_MCC V1.0\n\nTime Code Rate=30DF\n\n");
    assert_eq!(format_of(&mut session, &path), "mcc", "the file is an MCC file");
}

/// A file that is **not a caption file** is refused by name.
#[test]
fn a_file_that_is_not_a_caption_file_is_refused() {
    let mut session = session();
    let path = a_file("the-text.srt", "just some text\n");
    let error =
        session.execute("film.captions.detect", &json!({ "path": path })).expect_err("the file is refused");
    let message = error.to_string();
    assert!(message.contains("not a caption file"), "the file is refused by name: {message}");
}

/// A file whose **extension** is not one of a caption file is not a
/// caption file at all.
#[test]
fn a_file_with_an_extension_that_is_not_one_is_not_a_caption_file() {
    let mut session = session();
    let path = a_file("the-captions.txt", "1\n00:00:01,000 --> 00:00:02,000\nHello\n");
    let error =
        session.execute("film.captions.detect", &json!({ "path": path })).expect_err("the file is refused");
    let message = error.to_string();
    assert!(message.contains("not a caption file"), "the file is refused by name: {message}");
}

/// The **import** detects the format of the file when it is not named.
#[test]
fn the_import_detects_the_format_when_it_is_not_named() {
    let mut session = session();
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "25" }))
        .expect("the rate is set");
    let path = a_file(
        "the-import.scc",
        "Scenarist_SCC V1.0\n\n00:00:00;20\t9420 9420 94ae 94ae 94d0 94d0 c845 4c4c 4f80 942f 942f\n\n00:00:03;00\t942c 942c\n",
    );
    let outcome = session
        .execute("film.caption.import", &json!({ "sequence": sequence, "path": path }))
        .expect("the file is imported");
    assert_eq!(outcome.data["cues"], json!(1), "and its caption is imported: {outcome:?}");
}
