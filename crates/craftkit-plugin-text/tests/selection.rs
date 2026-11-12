//! The **selection of characters**: the part of a paragraph that is
//! picked out.
//!
//! A selection holds a part of a node, and a part is in the node's own
//! units. A paragraph's own unit is the **character**, so the SDK's own
//! span selection is a range of characters — and the paragraph says
//! where those characters are.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, RecordingCanvas, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_text::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the text battery.
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

/// A **monospaced** paragraph of that text, measured at that width.
///
/// The metric is two millimetres for every character and two for the
/// space, so a character is two millimetres.
fn a_paragraph(session: &mut Session, text: &str, measure_mm: f64) -> u64 {
    session
        .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 2.0 }))
        .expect("the metric is added");
    session
        .execute("text.paragraph.add", &json!({ "text": text, "metric": 1 }))
        .expect("the paragraph is added");
    let paragraph = session
        .document()
        .walk()
        .filter(|node| node.kind == "text.paragraph")
        .last()
        .map(|node| node.id)
        .expect("the paragraph is there");
    session
        .execute("text.paragraph.measure", &json!({ "paragraph": paragraph, "measure_mm": measure_mm }))
        .expect("the paragraph is measured");
    paragraph
}

/// The **parts** of the paragraph: where the characters from `from` to
/// `to` are.
fn parts(session: &Session, paragraph: u64, from: f64, to: f64) -> Vec<craftkit_sdk::Rect> {
    let node = session.document().node(paragraph).expect("the paragraph is there");
    session.parts_of(node, from, to)
}

/// The characters of a paragraph are picked out with the SDK's own span
/// selection: the paragraph's unit is the character.
#[test]
fn the_characters_are_picked_out_with_the_span_selection() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("select.span", &json!({ "node": paragraph, "from": 0, "to": 3 }))
        .expect("the characters are picked out");
    let selection = session.selection();
    assert_eq!(selection.len(), 1, "one thing is picked out");
    assert_eq!(selection[0].node, paragraph);
    assert_eq!(selection[0].from, Some(0.0));
    assert_eq!(selection[0].to, Some(3.0));
}

/// The picked-out characters are **where they are**: the paragraph says.
#[test]
fn the_picked_out_characters_are_where_they_are() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    let pieces = parts(&session, paragraph, 0.0, 3.0);
    assert_eq!(pieces.len(), 1, "the range is on one line");
    assert!((pieces[0].x - 0.0).abs() < 1e-9, "the first character is at the margin");
    assert!((pieces[0].w - 6.0).abs() < 1e-9, "three characters are six millimetres");
}

/// A range that **crosses a line** is more than one piece: one on each
/// line.
#[test]
fn a_range_that_crosses_a_line_is_more_than_one_piece() {
    let mut session = session();
    // "one two" is fourteen millimetres and "three" does not fit beside
    // it, so the paragraph is two lines.
    let paragraph = a_paragraph(&mut session, "one two three", 14.0);
    let pieces = parts(&session, paragraph, 4.0, 9.0);
    assert_eq!(pieces.len(), 2, "the range is on two lines");
    assert!((pieces[0].x - 8.0).abs() < 1e-9, "the first piece is where the word starts");
    assert!((pieces[1].x - 0.0).abs() < 1e-9, "and the second is at the margin of the next line");
}

/// The picked-out characters are **washed over**: the chrome marks the
/// part, and the part is not a frame.
#[test]
fn the_picked_out_characters_are_washed_over() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("select.span", &json!({ "node": paragraph, "from": 0, "to": 3 }))
        .expect("the characters are picked out");
    // The picture without the selection, to compare with.
    let mut plain = RecordingCanvas::default();
    session.paint(&mut plain);
    let mut canvas = RecordingCanvas::default();
    session.paint_view(&mut canvas);
    assert!(
        canvas.drawn.iter().any(|drawn| drawn.starts_with("rect(0, ") && drawn.ends_with(", 6, 5)")),
        "the characters are washed over: {:?}",
        canvas.drawn
    );
    let paths = canvas.drawn.iter().filter(|drawn| drawn.starts_with("path(")).count();
    assert_eq!(paths, plain.drawn.len(), "the letters are drawn as they were: a part adds no frame");
}

/// The **picture of the document** has no selection in it: the
/// selection is the screen's.
#[test]
fn a_picture_of_the_document_has_no_selection_in_it() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    session
        .execute("select.span", &json!({ "node": paragraph, "from": 0, "to": 3 }))
        .expect("the characters are picked out");
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    assert!(
        !canvas.drawn.iter().any(|drawn| drawn.starts_with("rect(")),
        "the wash is not in the picture: {:?}",
        canvas.drawn
    );
}

/// A range that is not a range of characters is not a part: the
/// paragraph says nothing about it.
#[test]
fn a_range_that_is_not_a_range_of_characters_is_not_a_part() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two", 14.0);
    assert!(parts(&session, paragraph, 0.5, 3.0).is_empty(), "half a character is not a character");
    // A range that runs past the end of the text picks out what is
    // there: the part is the characters, and there are no more.
    let pieces = parts(&session, paragraph, 0.0, 100.0);
    assert_eq!(pieces.len(), 1, "the whole text is one piece");
    assert!((pieces[0].w - 14.0).abs() < 1e-9, "and it is the whole line");
}
