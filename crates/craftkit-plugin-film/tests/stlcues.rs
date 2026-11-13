//! The **cues** of an **EBU STL** file.
//!
//! The file is binary: a general block and then the blocks that hold
//! the subtitles.

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
fn a_file(name: &str, bytes: &[u8]) -> String {
    let directory = craftkit_scratch::dir(&format!("craftkit-v2-stl-{}", std::process::id()))
        .expect("the directory is made");
    let path = directory.join(name);
    std::fs::write(&path, bytes).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// The **general block** of a file: the rate, and the start of the
/// programme.
fn general(rate: &str, start: &str) -> Vec<u8> {
    let mut block = vec![0u8; 1024];
    block[3..11].copy_from_slice(rate.as_bytes());
    block[256..256 + start.len()].copy_from_slice(start.as_bytes());
    block
}

/// A **block** of a subtitle: its number, its times and its text.
fn block(number: u16, extension: u8, tin: [u8; 4], tout: [u8; 4], comment: u8, text: &str) -> Vec<u8> {
    let mut bytes = vec![0u8; 128];
    bytes[1..3].copy_from_slice(&number.to_le_bytes());
    bytes[3] = extension;
    bytes[5..9].copy_from_slice(&tin);
    bytes[9..13].copy_from_slice(&tout);
    bytes[15] = comment;
    let text = text.as_bytes();
    let at = 16;
    bytes[at..at + text.len().min(112)].copy_from_slice(&text[..text.len().min(112)]);
    bytes
}

/// The **cues** of a file, as the answer holds them.
fn cues_of(session: &mut Session, path: &str) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "stl" }))
        .expect("the cues are read");
    outcome.data["cues"].as_array().expect("the cues are a list").clone()
}

/// An STL file is read into its **cues**: the times and the text.
#[test]
fn an_stl_file_is_read_into_its_cues() {
    let mut session = session();
    let mut bytes = general("STL25.01", "00000000");
    bytes.extend(block(1, 0xff, [0, 0, 1, 0], [0, 0, 2, 0], 0, "Hello there"));
    let path = a_file("basic.stl", &bytes);
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one cue: {cues:?}");
    assert_eq!(cues[0]["start"], json!(1000), "the start is a second: {cues:?}");
    assert_eq!(cues[0]["end"], json!(2000), "and the end: {cues:?}");
    assert_eq!(cues[0]["text"], json!("Hello there"), "and the text: {cues:?}");
}

/// The timecodes of the cues are **relative to the start of the
/// programme**: the tree's own rule.
#[test]
fn the_timecodes_are_relative_to_the_start_of_the_programme() {
    let mut session = session();
    // The programme starts at ten seconds.
    let mut bytes = general("STL25.01", "00001000");
    bytes.extend(block(1, 0xff, [0, 0, 12, 0], [0, 0, 14, 0], 0, "Later"));
    let path = a_file("tcp.stl", &bytes);
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one cue: {cues:?}");
    assert_eq!(cues[0]["start"], json!(2000), "the cue is two seconds in: {cues:?}");
    assert_eq!(cues[0]["end"], json!(4000), "and it lasts two: {cues:?}");
}

/// A subtitle that runs over **several blocks** is one cue.
#[test]
fn a_subtitle_over_several_blocks_is_one_cue() {
    let mut session = session();
    let mut bytes = general("STL25.01", "00000000");
    bytes.extend(block(1, 0x00, [0, 0, 1, 0], [0, 0, 3, 0], 0, "First half"));
    bytes.extend(block(1, 0xff, [0, 0, 1, 0], [0, 0, 3, 0], 0, " and the rest"));
    let path = a_file("blocks.stl", &bytes);
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one cue: {cues:?}");
    assert_eq!(cues[0]["text"], json!("First half and the rest"), "and the text is all of it: {cues:?}");
}

/// The **italics** and the **underline** are in the text, and a
/// **comment** block and a **user-data** block are skipped.
#[test]
fn the_italics_are_in_the_text_and_the_skipped_blocks_are_skipped() {
    let mut session = session();
    let mut bytes = general("STL25.01", "00000000");
    // A **comment** block: it is skipped.
    bytes.extend(block(1, 0x00, [0, 0, 1, 0], [0, 0, 2, 0], 1, "A comment"));
    // A **user-data** block: it is skipped.
    bytes.extend(block(2, 0xfe, [0, 0, 1, 0], [0, 0, 2, 0], 0, "User data"));
    // The **italics**: 0x80 opens them and 0x81 closes them.
    let mut text = vec![0x80u8];
    text.extend_from_slice(b"Italic");
    text.push(0x81);
    let mut one = vec![0u8; 128];
    one[1..3].copy_from_slice(&1u16.to_le_bytes());
    one[3] = 0xff;
    one[5..9].copy_from_slice(&[0, 0, 1, 0]);
    one[9..13].copy_from_slice(&[0, 0, 2, 0]);
    one[16..16 + text.len()].copy_from_slice(&text);
    bytes.extend(one);
    let path = a_file("italics.stl", &bytes);
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one cue: {cues:?}");
    assert_eq!(cues[0]["text"], json!("<i>Italic</i>"), "the italics are in the text: {cues:?}");
}

/// A file that is **not an STL file** is refused by name.
#[test]
fn a_file_that_is_not_an_stl_file_is_refused() {
    let mut session = session();
    let path = a_file("notstl.stl", b"this is not an STL file at all, and it is short");
    let outcome = session.execute("film.captions.read", &json!({ "path": path, "format": "stl" }));
    assert!(outcome.is_err(), "the file is refused: {outcome:?}");
}
