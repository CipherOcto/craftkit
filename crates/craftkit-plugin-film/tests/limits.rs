//! The **limits of the film battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. The numbers that come in are
//! from outside — a command line, a script, an agent — so they are
//! checked before they are used, and a limit that no test names is a
//! limit nothing proves holds.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
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

/// A caption file with **more cues** than `MAX_CUES` allows is refused.
///
/// The cues come from outside, and a list of ten thousand and one of them
/// is refused rather than written.
#[test]
fn a_caption_file_with_too_many_cues_is_refused() {
    let mut session = session();
    let path = craftkit_scratch::file("captions.srt").expect("the scratch file is there");
    let cues: Vec<craftkit_sdk::serde_json::Value> =
        (0..=10_000).map(|_| json!({ "start": 0, "end": 1, "text": "a cue" })).collect();
    let error = session
        .execute("film.captions.write", &json!({ "path": path, "cues": cues }))
        .expect_err("there are too many cues");
    assert!(error.to_string().contains("the most there may be"), "{error}");
}

/// A cue whose **text** is over `MAX_TEXT` long is refused.
#[test]
fn a_cue_with_too_much_text_is_refused() {
    let mut session = session();
    let path = craftkit_scratch::file("captions.srt").expect("the scratch file is there");
    let text = "t".repeat(4097);
    let cues = vec![json!({ "start": 0, "end": 1, "text": text })];
    let error = session
        .execute("film.captions.write", &json!({ "path": path, "cues": cues }))
        .expect_err("that text is too long");
    assert!(error.to_string().contains("longer than"), "{error}");
}

/// A cue that is **later than a year** is refused.
///
/// The time of a cue is a whole number of milliseconds, and the tree
/// says how far a caption may reach.
#[test]
fn a_cue_that_is_too_late_is_refused() {
    let mut session = session();
    let path = craftkit_scratch::file("captions.srt").expect("the scratch file is there");
    let cues = vec![json!({ "start": 0, "end": 31_536_000_001i64, "text": "a cue" })];
    let error = session
        .execute("film.captions.write", &json!({ "path": path, "cues": cues }))
        .expect_err("that cue is too late");
    assert!(error.to_string().contains("not between zero and a year"), "{error}");
}
