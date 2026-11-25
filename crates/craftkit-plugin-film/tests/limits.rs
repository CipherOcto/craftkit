//! The **limits of the film battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. The numbers that come in are
//! from outside — a command line, a script, an agent — so they are
//! checked before they are used, and a limit that no test names is a
//! limit nothing proves holds.
//!
//! Every cap of the battery is **named in code** here: the test says
//! the name of the cap and uses its number, so a cap that changes is a
//! cap that changes the test and not only the prose.

use craftkit_plugin_film::{
    MAX_CAPTION_CUES, MAX_FILM_CAPTION_TEXT, MAX_MILLISECONDS, MAX_TRACK_CUES, MAX_TRACK_TEXT,
};
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

/// A **sequence** with a **caption track** on it, and the track's id.
fn a_track(session: &mut Session) -> u64 {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    let outcome = session
        .execute("film.caption.track.add", &json!({ "sequence": sequence, "name": "the captions" }))
        .expect("the caption track is added");
    outcome.data["id"].as_u64().expect("the track has an id")
}

/// A **sequence** that runs at **25** frames a second.
fn a_sequence(session: &mut Session) -> u64 {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "25" }))
        .expect("the rate is set");
    sequence
}

/// The **time** of a cue in a caption file, as the files write it.
fn stamp(milliseconds: usize) -> String {
    let seconds = milliseconds / 1000;
    format!("{:02}:{:02}:{:02},{:03}", seconds / 3600, (seconds / 60) % 60, seconds % 60, milliseconds % 1000)
}

/// A caption file of **so many cues**, and its path.
///
/// The cues are a second apart, so that no two of them overlap.
fn a_file_of_cues(count: usize) -> String {
    let directory = craftkit_scratch::dir(&format!("craftkit-v2-filmlimits-{}", std::process::id()))
        .expect("the directory is made");
    let path = directory.join("the-cues.srt");
    let mut text = String::new();
    for index in 0..count {
        let start = index * 1000;
        text.push_str(&format!("{}\n{} --> {}\na cue\n\n", index + 1, stamp(start), stamp(start + 500)));
    }
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// A caption file with **more cues** than `MAX_CAPTION_CUES` allows is
/// refused.
///
/// The cues come from outside, and a list of ten thousand and one of them
/// is refused rather than written.
#[test]
fn a_caption_file_with_too_many_cues_is_refused() {
    let mut session = session();
    let path = craftkit_scratch::file("captions.srt").expect("the scratch file is there");
    let cues: Vec<craftkit_sdk::serde_json::Value> =
        (0..=MAX_CAPTION_CUES).map(|_| json!({ "start": 0, "end": 1, "text": "a cue" })).collect();
    let error = session
        .execute("film.captions.write", &json!({ "path": path, "cues": cues }))
        .expect_err("there are too many cues");
    let message = error.to_string();
    assert!(message.contains("the most there may be"), "{message}");
    assert!(message.contains(&MAX_CAPTION_CUES.to_string()), "the refusal names the bound: {message}");
}

/// A cue whose **text** is over `MAX_FILM_CAPTION_TEXT` long is refused.
#[test]
fn a_cue_with_too_much_text_is_refused() {
    let mut session = session();
    let path = craftkit_scratch::file("captions.srt").expect("the scratch file is there");
    let text = "t".repeat(MAX_FILM_CAPTION_TEXT + 1);
    let cues = vec![json!({ "start": 0, "end": 1, "text": text })];
    let error = session
        .execute("film.captions.write", &json!({ "path": path, "cues": cues }))
        .expect_err("that text is too long");
    let message = error.to_string();
    assert!(message.contains("longer than"), "{message}");
    assert!(message.contains(&MAX_FILM_CAPTION_TEXT.to_string()), "the refusal names the bound: {message}");
}

/// A cue that is **later than a year** is refused.
///
/// The time of a cue is a whole number of milliseconds, and the tree
/// says how far a caption may reach: a cue's time may be at most
/// `MAX_MILLISECONDS`, which is a year.
#[test]
fn a_cue_that_is_too_late_is_refused() {
    let mut session = session();
    let path = craftkit_scratch::file("captions.srt").expect("the scratch file is there");
    let cues = vec![json!({ "start": 0, "end": MAX_MILLISECONDS + 1, "text": "a cue" })];
    let error = session
        .execute("film.captions.write", &json!({ "path": path, "cues": cues }))
        .expect_err("that cue is too late");
    assert!(error.to_string().contains("not between zero and a year"), "{error}");
}

/// A cue whose **start** is over `MAX_MILLISECONDS` is refused.
///
/// The cap is on the start as well as on the end: a cue that begins
/// after a year begins later than the tree will reach.
#[test]
fn a_cue_that_starts_too_late_is_refused() {
    let mut session = session();
    let path = craftkit_scratch::file("captions.srt").expect("the scratch file is there");
    let cues = vec![json!({ "start": MAX_MILLISECONDS + 1, "end": MAX_MILLISECONDS + 2, "text": "a cue" })];
    let error = session
        .execute("film.captions.write", &json!({ "path": path, "cues": cues }))
        .expect_err("that cue starts too late");
    assert!(error.to_string().contains("the `start` of the cue 0 is not between zero and a year"), "{error}");
}

/// A cue whose **start** is **before zero** is refused.
///
/// The time of a cue is a whole number of milliseconds from the start
/// of the programme, and a time before the start is not a time.
#[test]
fn a_cue_that_starts_before_zero_is_refused() {
    let mut session = session();
    let path = craftkit_scratch::file("captions.srt").expect("the scratch file is there");
    let cues = vec![json!({ "start": -1i64, "end": 1i64, "text": "a cue" })];
    let error = session
        .execute("film.captions.write", &json!({ "path": path, "cues": cues }))
        .expect_err("that cue starts before the programme");
    assert!(error.to_string().contains("the `start` of the cue 0 is not between zero and a year"), "{error}");
}

/// A cue that ends **exactly** at `MAX_MILLISECONDS` is kept.
///
/// The cap is the last time a cue may have and not the first time it
/// may not: a year is a year, and a cue that ends there is a cue.
#[test]
fn a_cue_that_ends_at_the_limit_is_kept() {
    let mut session = session();
    let path = craftkit_scratch::file("captions.srt").expect("the scratch file is there");
    let cues = vec![json!({ "start": 0, "end": MAX_MILLISECONDS, "text": "a cue" })];
    session
        .execute("film.captions.write", &json!({ "path": path, "cues": cues }))
        .expect("a cue of a year is a cue");
    assert!(path.exists(), "and the file is written: {path:?}");
}

/// A cue of a **track** whose text is over `MAX_TRACK_TEXT` long is
/// refused.
///
/// The cap of a cue of a **file** and the cap of a cue of a **track**
/// are two caps, and they are two names: one is about the file that is
/// written and the other about the track that is held.
#[test]
fn a_track_cue_with_too_much_text_is_refused() {
    let mut session = session();
    let track = a_track(&mut session);
    let text = "t".repeat(MAX_TRACK_TEXT + 1);
    let error = session
        .execute("film.caption.add", &json!({ "track": track, "start": 0, "end": 1, "text": text }))
        .expect_err("that text is too long");
    let message = error.to_string();
    assert!(message.contains("the text of a cue is not longer than"), "{message}");
    assert!(message.contains(&MAX_TRACK_TEXT.to_string()), "the refusal names the bound: {message}");
}

/// A track that holds **more cues** than `MAX_TRACK_CUES` allows is
/// refused.
///
/// The cues of the track are put there **in one go**, by importing a
/// file that holds as many cues as a track may hold: a cue at a time
/// would be ten thousand commands, and the point of the test is the
/// **last** cue and not the way to it.
#[test]
fn a_track_with_too_many_cues_is_refused() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    let path = a_file_of_cues(MAX_TRACK_CUES);
    let outcome = session
        .execute("film.caption.import", &json!({ "sequence": sequence, "path": path }))
        .expect("the file is imported");
    let track = outcome.data["track"].as_u64().expect("the track has an id");
    let error = session
        .execute("film.caption.add", &json!({ "track": track, "start": 0, "end": 1, "text": "a cue" }))
        .expect_err("there are too many cues");
    let message = error.to_string();
    assert!(message.contains("a track holds at most"), "{message}");
    assert!(message.contains(&MAX_TRACK_CUES.to_string()), "the refusal names the bound: {message}");
}
