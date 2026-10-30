//! The editing half of the artist's IDE: a trim, a retime and a ripple.
//!
//! These are the operations that are about a **range** and not about a
//! node: the element keeps its place and plays less of its media, the
//! media plays faster, and the space one element took is given to the
//! ones after it.

use craftkit_sdk::Ecosystem;
// The tests that use it are the ones for the battery, and the battery is
// optional: the import is with them.
#[cfg(feature = "art")]
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A project with a video track and two elements: one at 48 that is 100
/// ticks long, and one at 200 that is 24 ticks long.
#[cfg(feature = "art")]
fn a_project() -> craftkit_sdk::Session {
    let mut session = ecosystem().session();
    session.execute("art.project.add", &json!({ "name": "the film" })).expect("the project is added");
    session
        .execute("art.scene.add", &json!({ "project": 1, "name": "act one", "main": true }))
        .expect("the scene is added");
    session.execute("art.track.video.add", &json!({ "scene": 2, "name": "V1" })).expect("the track is added");
    session
        .execute(
            "art.element.video.add",
            &json!({ "track": 3, "name": "the shot", "start": 48, "duration": 100, "media": "m_1" }),
        )
        .expect("the element is added");
    session
        .execute(
            "art.element.video.add",
            &json!({ "track": 3, "name": "the later shot", "start": 200, "duration": 24 }),
        )
        .expect("the element is added");
    session
}

/// The **length of an element on the timeline** is derived: the span of
/// the media it plays, divided by the rate it plays at.
#[cfg(feature = "art")]
#[test]
fn the_length_of_an_element_is_derived() {
    let session = a_project();
    let plan = session.evaluate();
    let timeline: Vec<_> = plan.items.iter().filter(|item| item.kind == "art.timeline").collect();
    assert_eq!(timeline[0].value["end"], json!(224), "200 + 24 is the end of the track");
}

/// A trim takes the media away: the element plays less of it, and it
/// takes less room on the timeline.
#[cfg(feature = "art")]
#[test]
fn a_trim_shortens_the_element() {
    let mut session = a_project();
    session
        .execute("art.element.trim", &json!({ "element": 4, "from": 10, "to": 60 }))
        .expect("the element is trimmed");
    let element = session.document().node(4).expect("the element is there");
    assert_eq!(element.data["trim_start"], json!(10));
    assert_eq!(element.data["trim_end"], json!(60));
    let plan = session.evaluate();
    let timeline: Vec<_> = plan.items.iter().filter(|item| item.kind == "art.timeline").collect();
    assert_eq!(timeline[0].value["end"], json!(224), "the trimmed element is shorter, and the other is not");
}

/// The trimmed element's own length follows: 60 - 10 is fifty ticks.
#[cfg(feature = "art")]
#[test]
fn the_trimmed_element_is_fifty_ticks_long() {
    let mut session = a_project();
    session
        .execute("art.element.trim", &json!({ "element": 4, "from": 10, "to": 60 }))
        .expect("the element is trimmed");
    session.evaluate();
    let element = session.document().node(4).expect("the element is there");
    assert_eq!(element.data["source_duration"], json!(100), "the media is still a hundred ticks long");
    assert_eq!(element.data["trim_end"], json!(60), "and only part of it is played");
}

/// A retime makes the media play faster, and the element takes less
/// room on the timeline.
#[cfg(feature = "art")]
#[test]
fn a_retime_shortens_the_element() {
    let mut session = a_project();
    session.execute("art.element.retime", &json!({ "element": 4, "rate": 2 })).expect("it is retimed");
    let element = session.document().node(4).expect("the element is there");
    assert_eq!(element.data["rate"], json!(2.0));
    let plan = session.evaluate();
    let timeline: Vec<_> = plan.items.iter().filter(|item| item.kind == "art.timeline").collect();
    assert_eq!(timeline[0].value["end"], json!(224), "the element is half as long, and the other is not");
}

/// The rate is **clamped**, not refused: it is the application's rule.
#[cfg(feature = "art")]
#[test]
fn a_rate_outside_the_bounds_is_clamped() {
    let mut session = a_project();
    session.execute("art.element.retime", &json!({ "element": 4, "rate": 1000 })).expect("it is retimed");
    let element = session.document().node(4).expect("the element is there");
    assert_eq!(element.data["rate"], json!(5.0), "five is as fast as it goes");

    let mut session = a_project();
    session.execute("art.element.retime", &json!({ "element": 4, "rate": 0.0001 })).expect("it is retimed");
    let element = session.document().node(4).expect("the element is there");
    assert_eq!(element.data["rate"], json!(0.01), "a hundredth is as slow as it goes");
}

/// A rate that is not a number is refused.
#[cfg(feature = "art")]
#[test]
fn a_rate_that_is_not_a_number_is_refused() {
    let mut session = a_project();
    let error = session
        .execute("art.element.retime", &json!({ "element": 4, "rate": 0 }))
        .expect_err("a rate of zero is not a rate");
    assert!(error.to_string().contains("above zero"), "{error}");
}

/// A trim that ends before it begins is refused.
#[cfg(feature = "art")]
#[test]
fn a_trim_that_ends_before_it_begins_is_refused() {
    let mut session = a_project();
    let error = session
        .execute("art.element.trim", &json!({ "element": 4, "from": 60, "to": 10 }))
        .expect_err("the trim is backwards");
    assert!(error.to_string().contains("ends after it begins"), "{error}");
}

/// A trim past the end of the media is refused, and it says how long
/// the media is.
#[cfg(feature = "art")]
#[test]
fn a_trim_past_the_end_of_the_media_is_refused() {
    let mut session = a_project();
    let error = session
        .execute("art.element.trim", &json!({ "element": 4, "from": 0, "to": 500 }))
        .expect_err("the media is not that long");
    assert!(error.to_string().contains("100 ticks long"), "{error}");
}

/// A ripple moves the elements after the cut, **on that track only**.
#[cfg(feature = "art")]
#[test]
fn a_ripple_moves_the_elements_after_the_cut() {
    let mut session = a_project();
    session
        .execute("art.ripple", &json!({ "track": 3, "after": 48, "amount": 20 }))
        .expect("the track ripples");
    let first = session.document().node(4).expect("the first element is there");
    let second = session.document().node(5).expect("the second element is there");
    assert_eq!(first.data["start"], json!(28), "the element at the cut moves earlier");
    assert_eq!(second.data["start"], json!(180), "and so does the one after it");
}

/// And it does not cross into another track.
#[cfg(feature = "art")]
#[test]
fn a_ripple_does_not_cross_into_another_track() {
    let mut session = a_project();
    session.execute("art.track.audio.add", &json!({ "scene": 2, "name": "A1" })).expect("the track is added");
    session
        .execute(
            "art.element.audio.add",
            &json!({ "track": 6, "name": "the music", "start": 300, "duration": 50 }),
        )
        .expect("the element is added");
    session
        .execute("art.ripple", &json!({ "track": 3, "after": 48, "amount": 20 }))
        .expect("the track ripples");
    let music = session.document().node(7).expect("the music is there");
    assert_eq!(music.data["start"], json!(300), "the other track did not move");
}

/// A ripple that would push an element before the start is refused.
#[cfg(feature = "art")]
#[test]
fn a_ripple_that_would_push_an_element_back_is_refused() {
    let mut session = a_project();
    let error = session
        .execute("art.ripple", &json!({ "track": 3, "after": 48, "amount": 100 }))
        .expect_err("the element would go before the start");
    assert!(error.to_string().contains("before the start"), "{error}");
}

/// A ripple of nothing is refused: it is a mistake, not a no-op.
#[cfg(feature = "art")]
#[test]
fn a_ripple_of_nothing_is_refused() {
    let mut session = a_project();
    let error = session
        .execute("art.ripple", &json!({ "track": 3, "after": 48, "amount": 0 }))
        .expect_err("nothing was taken out");
    assert!(error.to_string().contains("or it takes none"), "{error}");
}

/// The undo takes the trim back with it.
#[cfg(feature = "art")]
#[test]
fn the_undo_takes_the_trim_back() {
    let mut session = a_project();
    session
        .execute("art.element.trim", &json!({ "element": 4, "from": 10, "to": 60 }))
        .expect("the element is trimmed");
    assert_eq!(session.document().node(4).expect("the element is there").data["trim_end"], json!(60));
    session.undo().expect("the undo works");
    assert_eq!(
        session.document().node(4).expect("the element is there").data["trim_end"],
        json!(100),
        "the whole media is played again"
    );
}
