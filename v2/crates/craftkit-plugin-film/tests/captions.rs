//! The **cues** of a **SubRip** file.
//!
//! The reader takes what real files contain: missing indexes, a point
//! for the mark, short fractions, missing hours, and cues that are not
//! separated by a blank line.

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
fn a_file(name: &str, bytes: &[u8]) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-srt-{name}-{}", std::process::id()));
    std::fs::write(&path, bytes).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// The **cues** of a file, as the answer holds them.
fn cues_of(session: &mut Session, path: &str) -> Vec<Json> {
    let outcome = session.execute("film.captions.read", &json!({ "path": path })).expect("the cues are read");
    outcome.data["cues"].as_array().expect("the cues are a list").clone()
}

/// A SubRip file is read into its **cues**: the times and the text.
#[test]
fn a_subrip_file_is_read_into_its_cues() {
    let mut session = session();
    let path = a_file(
        "basic",
        b"1\n00:00:01,000 --> 00:00:03,500\nFirst line\n<i>second line</i>\n\n2\n00:00:04,000 --> 00:00:05,000\nBye\n",
    );
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 2, "there are two cues: {cues:?}");
    assert_eq!(cues[0]["start"], Json::from(1000), "the first starts at a second: {cues:?}");
    assert_eq!(cues[0]["end"], Json::from(3500), "and ends at three and a half: {cues:?}");
    assert_eq!(cues[0]["startClock"], Json::from("00:00:01,000"), "and says so: {cues:?}");
    assert_eq!(cues[0]["text"], Json::from("First line\n<i>second line</i>"), "the text is kept: {cues:?}");
    assert_eq!(cues[1]["text"], Json::from("Bye"), "and the second cue: {cues:?}");
}

/// The reader takes what real files contain: no indexes, a point for
/// the mark, short fractions, no hours, and no blank line between the
/// cues.
#[test]
fn the_reader_takes_what_real_files_contain() {
    let mut session = session();
    let path = a_file(
        "sloppy",
        b"00:00:01.5 --> 00:00:02.25 X1:10 X2:20\nA\n7\n00:03,000 --> 00:04,000\nB\n00:00:05,000-->00:00:06,000\n42\n\n\n",
    );
    let cues = cues_of(&mut session, &path);
    let times: Vec<(i64, i64, String)> = cues
        .iter()
        .map(|cue| {
            (
                cue["start"].as_i64().unwrap_or_default(),
                cue["end"].as_i64().unwrap_or_default(),
                cue["text"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    assert_eq!(
        times,
        vec![(1500, 2250, "A".to_string()), (3000, 4000, "B".to_string()), (5000, 6000, "42".to_string()),],
        "the cues are as they are: {cues:?}"
    );
}

/// A cue that **ends before it starts** is skipped, and the answer
/// says so.
#[test]
fn a_cue_that_ends_before_it_starts_is_skipped() {
    let mut session = session();
    let path =
        a_file("inverted", b"1\n00:00:05,000 --> 00:00:04,000\nX\n\n2\n00:00:06,000 --> 00:00:07,000\nY\n");
    let outcome = session.execute("film.captions.read", &json!({ "path": path })).expect("the cues are read");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list");
    assert_eq!(cues.len(), 1, "the first cue is skipped: {outcome:?}");
    let warnings = outcome.data["warnings"].as_array().expect("the warnings are a list");
    assert_eq!(warnings.len(), 1, "and it is said: {outcome:?}");
}

/// A file that has **no cue** and holds something is not a SubRip
/// file, and that is refused by name.
#[test]
fn a_file_that_is_not_a_subrip_file_is_refused() {
    let mut session = session();
    let path = a_file("garbage", b"hello world\nthis is not srt\n");
    let outcome = session.execute("film.captions.read", &json!({ "path": path }));
    assert!(outcome.is_err(), "the file is refused: {outcome:?}");
}

/// A file that holds **nothing** is a file with no cues.
#[test]
fn a_file_that_holds_nothing_holds_no_cues() {
    let mut session = session();
    let path = a_file("empty", b"");
    let cues = cues_of(&mut session, &path);
    assert!(cues.is_empty(), "there are no cues: {cues:?}");
}

/// The **bytes** of a file are read the tree's way: a mark of the byte
/// order is taken off, and a file that is neither UTF-8 nor UTF-16 is
/// read as Windows-1252.
#[test]
fn the_bytes_of_a_file_are_read_the_trees_way() {
    let mut session = session();
    // A file with a mark of the byte order and a little text.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice("00:00:01,000 --> 00:00:02,000\nCaf\u{00E9}\n".as_bytes());
    let path = a_file("bom", &bytes);
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one cue: {cues:?}");
    assert_eq!(cues[0]["text"], Json::from("Caf\u{00E9}"), "and the mark is taken off: {cues:?}");

    // A file that is UTF-16 with a mark of the byte order.
    let text = "00:00:01,000 --> 00:00:02,000\nWide\n";
    let mut bytes = vec![0xFF, 0xFE];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let path = a_file("utf16", &bytes);
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one cue: {cues:?}");
    assert_eq!(cues[0]["text"], Json::from("Wide"), "and the text is read: {cues:?}");

    // A file that is neither: Windows-1252.
    let mut bytes = b"00:00:01,000 --> 00:00:02,000\n".to_vec();
    bytes.push(0x93);
    bytes.extend_from_slice(b"a quote\n");
    let path = a_file("cp1252", &bytes);
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one cue: {cues:?}");
    assert_eq!(cues[0]["text"], Json::from("\u{201C}a quote"), "and the byte is Windows-1252: {cues:?}");
}

/// The **line endings** of a file may be a line feed, a carriage
/// return and a line feed, or a carriage return alone.
#[test]
fn the_line_endings_of_a_file_are_taken() {
    let mut session = session();
    let path = a_file(
        "cr",
        b"1\r\n00:00:01,000 --> 00:00:02,000\r\nA\r\n\r\n2\r00:00:03,000 --> 00:00:04,000\rB\r\r",
    );
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 2, "there are two cues: {cues:?}");
    assert_eq!(cues[0]["text"], Json::from("A"), "the first: {cues:?}");
    assert_eq!(cues[1]["text"], Json::from("B"), "and the second: {cues:?}");
}
