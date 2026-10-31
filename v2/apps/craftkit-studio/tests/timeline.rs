//! The **timeline**: the sound document's own world.
//!
//! §19 found that a sound document has no place in it: a clip is at a
//! **time**, and a time is not a place. This is the arc that gives it
//! one — a place that is **derived**, so that it never goes stale.

use craftkit_sdk::document::RecordingCanvas;
use craftkit_sdk::serde_json::json;

/// The id of the clip the tests make.
fn add_clip(session: &mut craftkit_sdk::Session) -> u64 {
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there");
    session
        .execute("sound.clip.add", &json!({ "track": track.id, "start": 0, "length": 96000 }))
        .expect("the clip is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there")
}

/// A sound document is **drawn**.
///
/// The kit draws it: a clip is a bar, and the bar is where the clip is.
#[cfg(feature = "sound")]
#[test]
fn a_sound_document_is_drawn() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    add_clip(&mut session);
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    let bars = canvas.drawn.iter().filter(|drawn| drawn.starts_with("rect(")).count();
    assert_eq!(bars, 1, "the clip is one bar: {:?}", canvas.drawn);
}

/// A sound document is **framed**.
///
/// The bounds ask the kinds where their nodes are, so a document whose
/// geometry is derived is framed by what is really there.
#[cfg(feature = "sound")]
#[test]
fn a_sound_document_is_framed() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    add_clip(&mut session);
    let bounds = session.bounds().expect("the document has bounds");
    assert_eq!(bounds.x, 0.0, "the timeline starts at the beginning");
    assert_eq!(bounds.w, 2.0, "and the clip is two seconds long");
    assert_eq!(bounds.h, 1.0, "and the row is one unit tall");
}

/// The picture of a sound document is **the picture of the document**.
///
/// The snapshot frames what is there, and the clip is in it.
#[cfg(feature = "sound")]
#[test]
fn a_sound_document_has_a_picture() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    add_clip(&mut session);
    let path = std::env::temp_dir().join("craftkit-v2-timeline.png");
    let path = path.to_string_lossy().to_string();
    let outcome = session
        .execute("doc.snapshot", &json!({ "path": path, "width": 64, "height": 32 }))
        .expect("the picture is made");
    assert!(std::path::Path::new(&path).exists(), "the picture is on the disk: {:?}", outcome.data);
}

/// The **mixer** is a picture.
///
/// The buses are the rows under the tracks, and the sends are the lines
/// between them: the mixer is a **graph**, and this is the graph.
#[cfg(feature = "sound")]
#[test]
fn the_mixer_is_a_picture() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    session.execute("sound.bus.add", &json!({ "name": "mix" })).expect("the bus is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    let bus = session.document().node_by_name("mix").expect("the bus is there").id;
    session
        .execute("sound.send.add", &json!({ "track": track, "bus": bus }))
        .expect("the send is added");

    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    // The track's row, the bus's row and the line between them.
    let lines = canvas.drawn.iter().filter(|drawn| drawn.starts_with("path(")).count();
    assert_eq!(lines, 3, "the mixer is three lines: {:?}", canvas.drawn);

    // And the mixer is **below** the timeline.
    let bounds = session.bounds().expect("the document has bounds");
    assert_eq!(bounds.h, 2.0, "a track's row and a bus's row");
}
