//! The **burn-in** of a caption: the caption drawn into the frame.

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

/// The **picture** that was written, and its samples.
fn a_picture(path: &str) -> craftkit_png::Picture {
    let bytes = std::fs::read(path).expect("the picture is there");
    assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']), "it is a PNG");
    let limits = craftkit_png::Limits::default();
    craftkit_png::read(&bytes, &limits).expect("the picture reads")
}

/// The number of pixels of the picture that are **not** clear.
fn painted(picture: &craftkit_png::Picture) -> u64 {
    let channels = usize::from(picture.channels);
    if channels < 4 {
        return 0;
    }
    picture.samples.chunks(channels).filter(|pixel| pixel[3] > 0).count() as u64
}

/// **A caption is burnt into the frame, and the picture is written.**
#[test]
fn a_caption_is_burnt_into_the_frame_and_the_picture_is_written() {
    let mut session = session();
    let track = a_track(&mut session);
    let directory = craftkit_scratch::dir(&format!("craftkit-v2-burn-{}", std::process::id()))
        .expect("the directory is made");
    let to = directory.join("frame.png").to_string_lossy().to_string();
    let outcome = session
        .execute(
            "film.caption.burn",
            &json!({ "text": "Hello", "track": track, "width": 640.0, "height": 360.0, "to": to }),
        )
        .expect("the caption is burnt");
    assert!(outcome.data["bytes"].as_u64().unwrap_or_default() > 0, "the picture has bytes: {outcome:?}");
    let picture = a_picture(&to);
    assert_eq!(picture.width, 640, "the picture is as wide as the frame: {picture:?}");
    assert_eq!(picture.height, 360, "and as tall: {picture:?}");
    assert!(painted(&picture) > 0, "the letters are drawn: {picture:?}");
}

/// **The letters are where the layout says they are.**
#[test]
fn the_letters_are_where_the_layout_says_they_are() {
    let mut session = session();
    let track = a_track(&mut session);
    let directory = craftkit_scratch::dir(&format!("craftkit-v2-burn-place-{}", std::process::id()))
        .expect("the directory is made");
    let to = directory.join("frame.png").to_string_lossy().to_string();
    let outcome = session
        .execute(
            "film.caption.burn",
            &json!({ "text": "Hello", "track": track, "width": 640.0, "height": 360.0, "to": to }),
        )
        .expect("the caption is burnt");
    let top = outcome.data["top"].as_f64().expect("the block has a top");
    let picture = a_picture(&to);
    // The letters are near the bottom of the frame: the block hangs
    // from the bottom, a margin away from it.
    assert!(top > 360.0 * 0.7, "the caption is at the bottom: {outcome:?}");
    // And the ink is in the lower part of the frame: the caption hangs
    // from the bottom, a margin away from it.
    let channels = usize::from(picture.channels);
    let first_row = picture.height as usize * 2 / 3;
    let mut low = 0_u64;
    for row in first_row..picture.height as usize {
        for column in 0..picture.width as usize {
            let at = (row * picture.width as usize + column) * channels;
            if picture.samples.get(at + 3).is_some_and(|alpha| *alpha > 0) {
                low += 1;
            }
        }
    }
    assert!(low > 0, "the letters are low in the frame");
}

/// **The background of the style is under the letters.**
#[test]
fn the_background_of_the_style_is_under_the_letters() {
    let mut session = session();
    let track = a_track(&mut session);
    session
        .execute(
            "film.caption.track.style",
            &json!({ "track": track, "background": true, "background_colour": "#ff0000ff" }),
        )
        .expect("the style is set");
    let directory = craftkit_scratch::dir(&format!("craftkit-v2-burn-bg-{}", std::process::id()))
        .expect("the directory is made");
    let to = directory.join("frame.png").to_string_lossy().to_string();
    session
        .execute(
            "film.caption.burn",
            &json!({ "text": "Hello", "track": track, "width": 640.0, "height": 360.0, "to": to }),
        )
        .expect("the caption is burnt");
    let picture = a_picture(&to);
    let channels = usize::from(picture.channels);
    let mut red = 0_u64;
    for pixel in picture.samples.chunks(channels) {
        if pixel[3] > 0 && pixel[0] > 200 && pixel[1] < 60 && pixel[2] < 60 {
            red += 1;
        }
    }
    assert!(red > 0, "the background is red: {picture:?}");
}

/// **A caption with no words is refused.**
#[test]
fn a_caption_with_no_words_is_refused() {
    let mut session = session();
    let track = a_track(&mut session);
    let directory = craftkit_scratch::dir(&format!("craftkit-v2-burn-empty-{}", std::process::id()))
        .expect("the directory is made");
    let to = directory.join("frame.png").to_string_lossy().to_string();
    let refusal = session
        .execute(
            "film.caption.burn",
            &json!({ "text": "  ", "track": track, "width": 640.0, "height": 360.0, "to": to }),
        )
        .expect_err("the caption is refused");
    let message = refusal.to_string();
    assert!(message.contains("words"), "the refusal says so: {message}");
}

/// **A frame that is too big is refused.**
#[test]
fn a_frame_that_is_too_big_is_refused() {
    let mut session = session();
    let track = a_track(&mut session);
    let directory = craftkit_scratch::dir(&format!("craftkit-v2-burn-big-{}", std::process::id()))
        .expect("the directory is made");
    let to = directory.join("frame.png").to_string_lossy().to_string();
    let refusal = session
        .execute(
            "film.caption.burn",
            &json!({ "text": "Hello", "track": track, "width": 100000.0, "height": 100000.0, "to": to }),
        )
        .expect_err("the frame is refused");
    let message = refusal.to_string();
    assert!(message.contains("pixels"), "the refusal says so: {message}");
}
