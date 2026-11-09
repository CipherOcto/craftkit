//! The **style** of a caption track, and the **track** itself.
//!
//! A caption track carries a style and a language, and the flags of
//! the tree. The rules are the tree's own.

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

/// A **caption track** on a sequence, and its id.
fn a_track(session: &mut Session) -> u64 {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "25" }))
        .expect("the rate is set");
    let outcome = session
        .execute("film.caption.track.add", &json!({ "sequence": sequence, "name": "the captions" }))
        .expect("the caption track is added");
    outcome.data["id"].as_u64().expect("the track has an id")
}

/// The **style** of a track, as the document holds it.
fn style_of(session: &Session, track: u64) -> craftkit_sdk::serde_json::Value {
    session
        .document()
        .node(track)
        .and_then(|node| node.data.get("style"))
        .cloned()
        .unwrap_or(craftkit_sdk::serde_json::Value::Null)
}

/// The **style** of a caption track is set, and the tree's defaults
/// are the style a track has when it has none.
#[test]
fn the_style_of_a_track_is_set() {
    let mut session = session();
    let track = a_track(&mut session);
    let outcome = session
        .execute(
            "film.caption.track.style",
            &json!({ "track": track, "font": "Courier", "size": 40, "align": "left", "anchor": "top" }),
        )
        .expect("the style is set");
    assert_eq!(outcome.data["style"]["font"], json!("Courier"), "the font is set: {outcome:?}");
    let style = style_of(&session, track);
    assert_eq!(style["size"], json!(40.0), "the size is set: {style:?}");
    assert_eq!(style["align"], json!("left"), "the align is set: {style:?}");
    assert_eq!(style["anchor"], json!("top"), "the anchor is set: {style:?}");
    // The rest of the style is the tree's own.
    assert_eq!(style["background"], json!(true), "the box is drawn: {style:?}");
    assert_eq!(style["line_spacing"], json!(1.25), "the line spacing is the tree's: {style:?}");
}

/// A **reset** puts the style back as it was.
#[test]
fn a_reset_puts_the_style_back() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("film.caption.track.style", &json!({ "track": track, "size": 40, "font": "Courier" }))
        .expect("the style is set");
    session
        .execute("film.caption.track.style", &json!({ "track": track, "reset": true }))
        .expect("the style is reset");
    let style = style_of(&session, track);
    assert_eq!(style["font"], json!("Inter"), "the font is the tree's: {style:?}");
    assert_eq!(style["size"], json!(54.0), "and the size: {style:?}");
}

/// A number outside its range is **refused**, and an align and an
/// anchor that are not the tree's are refused by name.
#[test]
fn a_style_that_is_not_one_is_refused() {
    let mut session = session();
    let track = a_track(&mut session);
    let error = session
        .execute("film.caption.track.style", &json!({ "track": track, "size": 500 }))
        .expect_err("the size is refused");
    assert!(error.to_string().contains("`size` is between 4 and 400"), "it says why: {error}");
    let error = session
        .execute("film.caption.track.style", &json!({ "track": track, "margin": 0.9 }))
        .expect_err("the margin is refused");
    assert!(error.to_string().contains("`margin` is between 0 and 0.45"), "it says why: {error}");
    let error = session
        .execute("film.caption.track.style", &json!({ "track": track, "align": "middle" }))
        .expect_err("the align is refused");
    assert!(error.to_string().contains("unknown align"), "it says why: {error}");
    let error = session
        .execute("film.caption.track.style", &json!({ "track": track, "anchor": "centre" }))
        .expect_err("the anchor is refused");
    assert!(error.to_string().contains("unknown anchor"), "it says why: {error}");
    let error = session
        .execute("film.caption.track.style", &json!({ "track": track, "colour": "red" }))
        .expect_err("the colour is refused");
    assert!(error.to_string().contains("is not a colour"), "it says why: {error}");
}

/// The **track** carries a name, a format, a language and its flags.
#[test]
fn the_track_carries_its_name_and_its_language() {
    let mut session = session();
    let track = a_track(&mut session);
    let outcome = session
        .execute(
            "film.caption.track.set",
            &json!({ "track": track, "name": "the French", "format": "cea708", "language": "fr-CA", "enabled": false }),
        )
        .expect("the track is set");
    assert_eq!(outcome.data["name"], json!("the French"), "the name is set: {outcome:?}");
    assert_eq!(outcome.data["language"], json!("fr-CA"), "the language is set: {outcome:?}");
    let node = session.document().node(track).expect("the track is there");
    assert_eq!(node.data["format"], json!("cea708"), "the format is set");
    assert_eq!(node.data["enabled"], json!(false), "and the flag");
}

/// A format that is not one of the tree's is **refused** by name, and
/// a name and a language that say nothing are refused.
#[test]
fn a_track_that_is_not_one_is_refused() {
    let mut session = session();
    let track = a_track(&mut session);
    let error = session
        .execute("film.caption.track.set", &json!({ "track": track, "format": "srt" }))
        .expect_err("the format is refused");
    assert!(error.to_string().contains("is not a format of a caption track"), "it says why: {error}");
    let error = session
        .execute("film.caption.track.set", &json!({ "track": track, "name": "  " }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("has a name"), "it says why: {error}");
    let error = session
        .execute("film.caption.track.set", &json!({ "track": track, "language": "" }))
        .expect_err("the language is refused");
    assert!(error.to_string().contains("has a language"), "it says why: {error}");
}
