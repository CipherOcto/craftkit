//! The **cues** of a TTML file.
//!
//! TTML is XML, and the times of the captions are in it: a clock, an
//! offset, or a number of frames or ticks. The rules are the tree's
//! own.

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
    let directory = std::env::temp_dir().join(format!("craftkit-v2-ttmlread-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("the directory is made");
    let path = directory.join(name);
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// The **cues** of a file, as the answer holds them.
fn cues_of(session: &mut Session, path: &str) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "ttml" }))
        .expect("the file is read");
    outcome.data["cues"].as_array().expect("the cues are a list").clone()
}

/// The **warnings** of a file, as the answer holds them.
fn warnings_of(session: &mut Session, path: &str) -> Vec<String> {
    let outcome = session
        .execute("film.captions.read", &json!({ "path": path, "format": "ttml" }))
        .expect("the file is read");
    outcome.data["warnings"]
        .as_array()
        .expect("the warnings are a list")
        .iter()
        .filter_map(|warning| warning.as_str().map(str::to_string))
        .collect()
}

/// A **caption** is read: the times of it are the times of the file.
#[test]
fn a_caption_is_read_from_a_ttml_file() {
    let mut session = session();
    let path = a_file(
        "the-captions.ttml",
        r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><div><p begin="1s" end="2s">Hello</p></div></body></tt>"#,
    );
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one caption: {cues:?}");
    assert_eq!(cues[0]["text"], json!("Hello"), "and it says Hello: {cues:?}");
    assert_eq!(cues[0]["start"], json!(1000), "it begins at its time: {cues:?}");
    assert_eq!(cues[0]["end"], json!(2000), "and it ends at its own: {cues:?}");
}

/// The **timed spans** of a paragraph without a time of its own are the
/// captions.
#[test]
fn the_timed_spans_of_a_paragraph_are_the_captions() {
    let mut session = session();
    let path = a_file(
        "the-spans.ttml",
        r#"<tt><body><div><p><span begin="0s" end="1s">One</span><span begin="1s" end="2s">Two</span></p></div></body></tt>"#,
    );
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 2, "there are two captions: {cues:?}");
    assert_eq!(cues[0]["text"], json!("One"), "the first is One: {cues:?}");
    assert_eq!(cues[1]["text"], json!("Two"), "and the second is Two: {cues:?}");
    assert_eq!(cues[1]["start"], json!(1000), "and it begins at its time: {cues:?}");
}

/// A **container** clips what is inside it: nothing inside outlasts the
/// end of the container.
#[test]
fn a_container_clips_what_is_inside_it() {
    let mut session = session();
    let path = a_file(
        "the-clip.ttml",
        r#"<tt><body><div begin="0s" end="1s"><p begin="0s" end="5s">Clipped</p></div></body></tt>"#,
    );
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one caption: {cues:?}");
    assert_eq!(cues[0]["end"], json!(1000), "and it ends with its container: {cues:?}");
}

/// The **text** of a caption: the whitespace collapses to one space,
/// and a `<br/>` is a line break.
#[test]
fn the_text_of_a_caption_collapses_its_whitespace() {
    let mut session = session();
    let path = a_file(
        "the-text.ttml",
        r#"<tt><body><div><p begin="0s" end="1s">Hello   <br/>world</p></div></body></tt>"#,
    );
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one caption: {cues:?}");
    assert_eq!(cues[0]["text"], json!("Hello\nworld"), "and the text is as it is: {cues:?}");
}

/// The **tick rate** of the file is the tree's own: an absent tick
/// rate is the frame rate times the sub-frame rate.
#[test]
fn the_tick_rate_of_the_file_is_used() {
    let mut session = session();
    let path = a_file(
        "the-ticks.ttml",
        r#"<tt xmlns:ttp="http://www.w3.org/ns/ttml#parameter" ttp:tickRate="1000"><body><div><p begin="500t" end="1500t">Ticks</p></div></body></tt>"#,
    );
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one caption: {cues:?}");
    assert_eq!(cues[0]["start"], json!(500), "a tick is a millisecond: {cues:?}");
    assert_eq!(cues[0]["end"], json!(1500), "and there are five hundred of them: {cues:?}");
}

/// The **frame rate** of the file is the tree's own: the multiplier
/// makes the frame rate of the file.
#[test]
fn the_frame_rate_of_the_file_is_used() {
    let mut session = session();
    let path = a_file(
        "the-frames.ttml",
        r#"<tt xmlns:ttp="http://www.w3.org/ns/ttml#parameter" ttp:frameRate="25" ttp:frameRateMultiplier="2 1"><body><div><p begin="1f" end="2f">Frames</p></div></body></tt>"#,
    );
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "there is one caption: {cues:?}");
    // The frame rate is fifty frames a second, and a frame is twenty
    // milliseconds.
    assert_eq!(cues[0]["start"], json!(20), "a frame is twenty milliseconds: {cues:?}");
    assert_eq!(cues[0]["end"], json!(40), "and two of them are forty: {cues:?}");
}

/// A file that is **not a TTML file** is refused by name.
#[test]
fn a_file_that_is_not_a_ttml_file_is_refused() {
    let mut session = session();
    let path = a_file("not-ttml.ttml", "<html><body>no</body></html>");
    let error = session
        .execute("film.captions.read", &json!({ "path": path, "format": "ttml" }))
        .expect_err("the file is refused");
    let message = error.to_string();
    assert!(message.contains("TTML"), "the format is named: {message}");
}

/// A **time** that is not one is a warning, and the caption it is on is
/// left out.
#[test]
fn a_time_that_is_not_one_is_a_warning() {
    let mut session = session();
    let path = a_file(
        "the-bad-time.ttml",
        r#"<tt><body><div><p begin="soon" end="2s">Never</p><p begin="1s" end="2s">Always</p></div></body></tt>"#,
    );
    let cues = cues_of(&mut session, &path);
    assert_eq!(cues.len(), 1, "the caption with the good time is there: {cues:?}");
    assert_eq!(cues[0]["text"], json!("Always"), "and it is the right one: {cues:?}");
    let warnings = warnings_of(&mut session, &path);
    assert_eq!(warnings.len(), 1, "there is one warning: {warnings:?}");
    assert!(warnings[0].contains("bad time expression"), "and it says what is wrong: {warnings:?}");
}
