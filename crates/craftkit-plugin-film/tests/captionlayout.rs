//! The **layout** of a caption: where the caption goes on the frame.

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
    let outcome = session
        .execute("film.caption.track.add", &json!({ "sequence": sequence, "name": "the captions" }))
        .expect("the caption track is added");
    outcome.data["id"].as_u64().expect("the track has an id")
}

/// The **layout** of a caption on a frame.
fn layout(
    session: &mut Session,
    text: &str,
    track: Option<u64>,
    width: f64,
    height: f64,
    settings: Option<&str>,
) -> craftkit_sdk::serde_json::Value {
    let mut params = json!({ "text": text, "width": width, "height": height });
    if let Some(track) = track {
        params["track"] = json!(track);
    }
    if let Some(settings) = settings {
        params["settings"] = json!(settings);
    }
    let outcome = session.execute("film.caption.layout", &params).expect("the caption is laid out");
    outcome.data.clone()
}

/// **A caption hangs from the bottom of the frame, a margin away.**
#[test]
fn a_caption_hangs_from_the_bottom_of_the_frame_a_margin_away() {
    let mut session = session();
    let track = a_track(&mut session);
    let laid = layout(&mut session, "Hello", Some(track), 1920.0, 1080.0, None);
    // The size of the letters is the tree's own — fifty-four pixels on
    // a frame of a thousand and eighty lines.
    assert_eq!(laid["size"], json!(54.0), "the size is the tree's: {laid:?}");
    // The line is a hundred and twenty pixels high at the tree's own
    // line spacing of one and a quarter.
    assert_eq!(laid["line_height"], json!(67.5), "the line is that high: {laid:?}");
    // The margin is eight per cent of the height of the frame.
    assert_eq!(laid["margin"], json!(86.4), "the margin is the tree's: {laid:?}");
    // And the block hangs from the bottom, that far above it — and the
    // place is rounded, as the tree's is.
    assert_eq!(laid["top"], json!(926.0), "the top is that: {laid:?}");
}

/// **The size of the letters is relative to the height of the frame.**
#[test]
fn the_size_of_the_letters_is_relative_to_the_height_of_the_frame() {
    let mut session = session();
    let track = a_track(&mut session);
    let small = layout(&mut session, "Hello", Some(track), 960.0, 540.0, None);
    assert_eq!(small["size"], json!(27.0), "half the frame, half the size: {small:?}");
    let large = layout(&mut session, "Hello", Some(track), 3840.0, 2160.0, None);
    assert_eq!(large["size"], json!(108.0), "twice the frame, twice the size: {large:?}");
}

/// **The lines are wrapped at ninety per cent of the frame.**
#[test]
fn the_lines_are_wrapped_at_ninety_per_cent_of_the_frame() {
    let mut session = session();
    let track = a_track(&mut session);
    let words = "one two three four five six seven eight nine ten eleven twelve";
    let laid = layout(&mut session, words, Some(track), 1920.0, 1080.0, None);
    let lines = laid["lines"].as_array().expect("the lines are there");
    assert!(lines.len() > 1, "the text is wrapped: {laid:?}");
    for line in lines {
        let line = line.as_str().unwrap_or_default();
        assert!(!line.is_empty(), "no line is empty: {laid:?}");
        assert!(!line.starts_with(' '), "no line begins with a space: {laid:?}");
    }
    // Every line is inside the frame.
    let width = 1920.0;
    for x in laid["x"].as_array().expect("the places are there") {
        let x = x.as_f64().unwrap_or_default();
        assert!(x >= 0.0 && x <= width, "the line is inside the frame: {laid:?}");
    }
}

/// **The align of the style says where the lines are.**
#[test]
fn the_align_of_the_style_says_where_the_lines_are() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute("film.caption.track.style", &json!({ "track": track, "align": "left", "anchor": "bottom" }))
        .expect("the style is set");
    let left = layout(&mut session, "Hello", Some(track), 1920.0, 1080.0, None);
    let left_x = left["x"][0].as_f64().expect("the line has a place");
    session
        .execute("film.caption.track.style", &json!({ "track": track, "align": "right", "anchor": "bottom" }))
        .expect("the style is set");
    let right = layout(&mut session, "Hello", Some(track), 1920.0, 1080.0, None);
    let right_x = right["x"][0].as_f64().expect("the line has a place");
    assert!(left_x < right_x, "the left line is further left: {left:?} {right:?}");
    // The left line is the side of the frame and the padding.
    assert_eq!(left_x, json!(96.0 + 16.0), "the side and the padding: {left:?}");
}

/// **The anchor of the style says where the block hangs.**
#[test]
fn the_anchor_of_the_style_says_where_the_block_hangs() {
    let mut session = session();
    let track = a_track(&mut session);
    let mut tops: Vec<f64> = Vec::new();
    for anchor in ["top", "middle", "bottom"] {
        session
            .execute("film.caption.track.style", &json!({ "track": track, "anchor": anchor }))
            .expect("the style is set");
        let laid = layout(&mut session, "Hello", Some(track), 1920.0, 1080.0, None);
        tops.push(laid["top"].as_f64().expect("the block has a top"));
    }
    assert!(tops[0] < tops[1] && tops[1] < tops[2], "the anchors are in order: {tops:?}");
    assert_eq!(tops[0], json!(86.0), "the top anchor is the margin: {tops:?}");
}

/// **The settings of the cue say where the caption goes.**
#[test]
fn the_settings_of_the_cue_say_where_the_caption_goes() {
    let mut session = session();
    let track = a_track(&mut session);
    let laid = layout(&mut session, "Hello", Some(track), 1920.0, 1080.0, Some("line:50% align:left"));
    assert_eq!(laid["top"], json!(540.0), "the line says where the block is: {laid:?}");
    assert_eq!(laid["align"], json!("left"), "and the align says where the line is: {laid:?}");
    let left_x = laid["x"][0].as_f64().expect("the line has a place");
    assert_eq!(left_x, json!(96.0 + 16.0), "the line is at the side: {laid:?}");
}

/// **A caption with no words is refused.**
#[test]
fn a_caption_with_no_words_is_refused() {
    let mut session = session();
    let track = a_track(&mut session);
    let refusal = session
        .execute(
            "film.caption.layout",
            &json!({ "text": "   ", "track": track, "width": 1920.0, "height": 1080.0 }),
        )
        .expect_err("the caption is refused");
    let message = refusal.to_string();
    assert!(message.contains("words"), "the refusal says so: {message}");
}

/// **A frame with no height is refused.**
#[test]
fn a_frame_with_no_height_is_refused() {
    let mut session = session();
    let track = a_track(&mut session);
    let refusal = session
        .execute(
            "film.caption.layout",
            &json!({ "text": "Hello", "track": track, "width": 1920.0, "height": 0.0 }),
        )
        .expect_err("the frame is refused");
    let message = refusal.to_string();
    assert!(message.contains("frame"), "the refusal says so: {message}");
}

/// **The outline grows the box of the caption.**
#[test]
fn the_outline_grows_the_box_of_the_caption() {
    let mut session = session();
    let track = a_track(&mut session);
    let plain = layout(&mut session, "Hello", Some(track), 1920.0, 1080.0, None);
    session
        .execute("film.caption.track.style", &json!({ "track": track, "outline": 4.0 }))
        .expect("the style is set");
    let outlined = layout(&mut session, "Hello", Some(track), 1920.0, 1080.0, None);
    let plain_box = plain["box"].as_array().expect("the box is there");
    let outlined_box = outlined["box"].as_array().expect("the box is there");
    assert!(
        outlined_box[0].as_f64().unwrap_or_default() < plain_box[0].as_f64().unwrap_or_default(),
        "the box grows to the left: {plain_box:?} {outlined_box:?}"
    );
    assert!(
        outlined_box[3].as_f64().unwrap_or_default() > plain_box[3].as_f64().unwrap_or_default(),
        "and downwards: {plain_box:?} {outlined_box:?}"
    );
}
