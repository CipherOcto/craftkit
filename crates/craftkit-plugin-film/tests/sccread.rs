//! The **cues** of an SCC file.
//!
//! An SCC file carries CEA-608 byte pairs with SMPTE timecodes, and
//! the captions they hold are read. The rules are the tree's own.

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
    let directory = std::env::temp_dir().join(format!("craftkit-v2-scc-read-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("the directory is made");
    let path = directory.join(name);
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// The **cues** of a file, as the answer holds them.
fn cues_of(session: &mut Session, path: &str) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "scc" }))
        .expect("the file is read");
    outcome.data["cues"].as_array().expect("the cues are a list").clone()
}

/// A **pop-on** caption is read: it is loaded, it is shown, and it is
/// cleared.
#[test]
fn a_pop_on_caption_is_read() {
    let mut session = session();
    let path = a_file(
        "popon.scc",
        "Scenarist_SCC V1.0\n\n00:00:00;20\t9420 9420 94ae 94ae 94d0 94d0 c845 4c4c 4f80 942f 942f\n\n00:00:03;00\t942c 942c\n",
    );
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one caption: {cues:?}");
    assert_eq!(cues[0]["text"], json!("HELLO"), "and it says HELLO: {cues:?}");
    // It is shown at the frame the end of caption arrives and it goes
    // when the displayed memory is erased.
    assert_eq!(cues[0]["start"], json!(967), "it is shown at the frame: {cues:?}");
    assert_eq!(cues[0]["end"], json!(3003), "and it goes: {cues:?}");
}

/// A **roll-up** caption is read: it rolls, and the rows it has rolled
/// are kept.
#[test]
fn a_roll_up_caption_is_read() {
    let mut session = session();
    let path = a_file(
        "rollup.scc",
        "Scenarist_SCC V1.0\n\n00:00:00:00\t9425 9425 94ad 94ad c1c2 94ad 94ad 4344\n\n00:00:02:00\t942c 942c\n",
    );
    let cues = cues_of(&mut session, &path);
    let texts: Vec<String> =
        cues.iter().map(|cue| cue["text"].as_str().unwrap_or_default().to_string()).collect();
    assert_eq!(texts, vec!["AB".to_string(), "AB\nCD".to_string()], "the rows are kept: {cues:?}");
}

/// A file that is **not** an SCC file is refused, and a **timecode**
/// that is not one is refused with the line it is on.
#[test]
fn a_file_that_is_not_one_is_refused() {
    let mut session = session();
    let path = a_file("other.scc", "Scenarist VTT V1.0\n\n00:00:00.000 --> 00:00:01.000\nHello\n");
    let error = session
        .execute("film.captions.read", &json!({ "path": path, "format": "scc" }))
        .expect_err("the file is refused");
    assert!(error.to_string().contains("not a Scenarist SCC file"), "it says why: {error}");
    let path = a_file("badtime.scc", "Scenarist_SCC V1.0\n\n00:00:xx;20\t9420\n");
    let error = session
        .execute("film.captions.read", &json!({ "path": path, "format": "scc" }))
        .expect_err("the timecode is refused");
    assert!(error.to_string().contains("the timecode"), "it says why: {error}");
}

/// A **code word** that is not one is a warning, and the file is read
/// without it.
#[test]
fn a_code_word_that_is_not_one_is_a_warning() {
    let mut session = session();
    let path =
        a_file("warning.scc", "Scenarist_SCC V1.0\n\n00:00:00;20\t9420 94ae zzzz c845 4c4c 4f80 942f 942f\n");
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "scc" }))
        .expect("the file is read");
    let warnings = outcome.data["warnings"].as_array().expect("the warnings are a list");
    assert_eq!(warnings.len(), 1, "there is one warning: {outcome:?}");
    assert!(warnings[0].as_str().unwrap_or_default().contains("zzzz"), "it names the word: {outcome:?}");
    let cues = outcome.data["cues"].as_array().expect("the cues are a list").clone();
    assert_eq!(cues.len(), 1, "and the caption is still read: {outcome:?}");
    assert_eq!(cues[0]["text"], json!("HELLO"), "with its text: {outcome:?}");
}

/// The **timecode** of the format: the drop-frame mark is read, and
/// the frames of a minute are two fewer.
#[test]
fn the_timecode_of_the_format_is_read() {
    let mut session = session();
    let path = a_file(
        "drop.scc",
        "Scenarist_SCC V1.0\n\n00:01:00;02\t9420 9420 94ae 94ae 94d0 94d0 c845 4c4c 4f80 942f 942f\n",
    );
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "the caption is read: {cues:?}");
    // The line stands at the frame 1800 — the drop-frame timecode of
    // a minute — and the caption is shown at the frame the end of
    // caption arrives, which is the tenth pair of the line.
    assert_eq!(cues[0]["start"], json!(60360), "and it is shown there: {cues:?}");
}
