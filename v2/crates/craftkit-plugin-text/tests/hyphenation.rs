//! The **hyphenation**: where a paragraph's words may be broken.
//!
//! §145 found that the breaker knew a hyphenation point but that
//! nobody could say where the points were: the caller had to, and the
//! caller had no dictionary. This is the arc that gives the kit the
//! patterns, so that the paragraph asks the kit and the kit answers.

use craftkit_sdk::serde_json::{Value, json};
use craftkit_sdk::{Ecosystem, Session};

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

/// A session with a metric and a paragraph of that text, measured at
/// that width.
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
        .find(|node| node.kind == "text.paragraph")
        .map(|node| node.id)
        .expect("the paragraph is there");
    session
        .execute("text.paragraph.measure", &json!({ "paragraph": paragraph, "measure_mm": measure_mm }))
        .expect("the paragraph is measured");
    paragraph
}

/// The **lines** of the paragraph, as the plan says them.
fn lines(session: &Session) -> Vec<Value> {
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.kind == "text.lines").expect("the lines are in the plan");
    item.value["lines"].as_array().expect("the lines are a list").clone()
}

/// A paragraph is **not** hyphenated until somebody says so.
///
/// A hyphen in a line is a thing a reader sees, and a reader who did
/// not ask for one did not get one.
#[test]
fn a_paragraph_is_not_hyphenated_until_it_is_asked_to_be() {
    let mut session = session();
    a_paragraph(&mut session, "hyphenation", 20.0);
    let lines = lines(&session);
    assert!(!lines.is_empty(), "the paragraph has lines: {lines:?}");
    assert!(
        lines.iter().any(|line| line["excess"].as_f64().unwrap_or(0.0) < 0.0),
        "and one of them is too long: {lines:?}"
    );
    assert!(
        lines.iter().all(|line| line["flagged"] == json!(false)),
        "and none of them ends at a hyphen: {lines:?}"
    );
}

/// A hyphenated paragraph **breaks** the word.
///
/// The measure is too narrow for the word, and the word is broken to
/// fit it.
#[test]
fn a_hyphenated_paragraph_breaks_the_word() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "hyphenation", 20.0);
    session
        .execute("text.paragraph.hyphenate", &json!({ "paragraph": paragraph, "hyphenate": true }))
        .expect("the paragraph hyphenates");
    let lines = lines(&session);
    assert_eq!(lines.len(), 2, "the word is broken over two lines: {lines:?}");
    assert!(lines[0]["excess"].as_f64().unwrap_or(0.0) >= 0.0, "and the first line fits: {}", lines[0]);
}

/// The line that ends at a hyphenation point is **flagged**.
///
/// The flag says the line ended at a break that carries a hyphen, and
/// a reader of the plan can tell a hyphenated line from a whole one.
#[test]
fn the_line_that_ends_at_a_hyphenation_point_is_flagged() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "hyphenation", 20.0);
    session
        .execute("text.paragraph.hyphenate", &json!({ "paragraph": paragraph, "hyphenate": true }))
        .expect("the paragraph hyphenates");
    let lines = lines(&session);
    assert_eq!(lines[0]["flagged"], json!(true), "the first line ended at a hyphen: {lines:?}");
}

/// The **words** of a line are the words of the paragraph.
///
/// A broken word is more than one item, and the plan still says which
/// words a line holds.
#[test]
fn the_words_of_a_line_are_the_words_of_the_paragraph() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "one two three", 40.0);
    session
        .execute("text.paragraph.hyphenate", &json!({ "paragraph": paragraph, "hyphenate": true }))
        .expect("the paragraph hyphenates");
    let lines = lines(&session);
    assert!(!lines.is_empty(), "the paragraph has lines: {lines:?}");
    for line in &lines {
        let start = line["start"].as_u64().expect("the line starts at a word");
        let end = line["end"].as_u64().expect("the line ends at a word");
        assert!(start < end, "the line holds at least one word: {line}");
        assert!(end <= 3, "and the paragraph has three words: {line}");
    }
}

/// The **hyphen** is drawn.
///
/// A line that ends at a hyphenation point ends in a hyphen, and the
/// picture says so: the drawing holds one more stroke than the same
/// paragraph without the hyphen.
#[test]
fn the_hyphen_is_drawn() {
    let mut session = session();
    let paragraph = a_paragraph(&mut session, "hyphenation", 20.0);
    let mut plain = craftkit_sdk::document::RecordingCanvas::default();
    session.paint(&mut plain);
    let strokes = |canvas: &craftkit_sdk::document::RecordingCanvas| {
        canvas.drawn.iter().filter(|drawn| drawn.starts_with("path(")).count()
    };
    let before = strokes(&plain);
    session
        .execute("text.paragraph.hyphenate", &json!({ "paragraph": paragraph, "hyphenate": true }))
        .expect("the paragraph hyphenates");
    let mut broken = craftkit_sdk::document::RecordingCanvas::default();
    session.paint(&mut broken);
    assert_eq!(strokes(&broken), before + 1, "the hyphen is one more stroke: {:?}", broken.drawn);
}

/// A node that is **not a paragraph** is refused.
#[test]
fn a_node_that_is_not_a_paragraph_is_refused() {
    let mut session = session();
    let refused = session.execute("text.paragraph.hyphenate", &json!({ "paragraph": 1, "hyphenate": true }));
    assert!(refused.is_err(), "there is no paragraph there");
}
