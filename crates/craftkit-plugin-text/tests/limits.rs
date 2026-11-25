//! The **limits of the text battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. The numbers that come in are
//! from outside — a command line, a script, an agent — so they are
//! checked before they are used, and a limit that no test names is a
//! limit nothing proves holds.

use craftkit_plugin_text::MAX_RUN;
use craftkit_sdk::serde_json::{Value, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_text::CRAFTKIT_PLUGIN_ANCHOR;

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

/// A paragraph of that text, in a metric whose runs are that table,
/// measured at a column wide enough to hold it on one line.
///
/// The metric is one millimetre for every character and one for the
/// space, so a word of eight characters that has no run is eight
/// millimetres wide.
fn a_paragraph(session: &mut Session, text: &str, runs: Value) -> u64 {
    let outcome = session
        .execute(
            "text.metric.add",
            &json!({ "name": "the metric", "default_mm": 1.0, "space_mm": 1.0, "runs": runs }),
        )
        .expect("the metric is added");
    let metric = outcome.data["id"].as_u64().expect("the metric has an id");
    let outcome = session
        .execute("text.paragraph.add", &json!({ "text": text, "metric": metric }))
        .expect("the paragraph is added");
    let paragraph = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session
        .execute("text.paragraph.measure", &json!({ "paragraph": paragraph, "measure_mm": 1000.0 }))
        .expect("the paragraph is measured");
    paragraph
}

/// The **width** of the paragraph's only line, as the plan says it.
fn the_width(session: &Session) -> f64 {
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.kind == "text.lines").expect("the lines are in the plan");
    let lines = item.value["lines"].as_array().expect("the lines are a list");
    assert_eq!(lines.len(), 1, "the paragraph is one line: {lines:?}");
    lines[0]["width"].as_f64().expect("the line has a width")
}

/// The number of the metric's runs that were **too long to be used**, as
/// the measurement says it.
fn runs_ignored(session: &Session) -> u64 {
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.kind == "text.lines").expect("the lines are in the plan");
    item.value["runs_ignored"].as_u64().expect("the measurement says how many runs were ignored")
}

/// A run of **exactly `MAX_RUN` characters** is used.
///
/// A run is looked for at every place of a word, and the longest one
/// that matches wins: eight characters is the longest run the metric
/// will look for, so a run of eight is a run.
#[test]
fn a_run_of_the_limit_is_used() {
    let run = "abcdefgh";
    assert_eq!(run.chars().count(), MAX_RUN, "the run is as long as the cap");
    let mut session = session();
    a_paragraph(&mut session, run, json!({ "abcdefgh": 100.0 }));
    let width = the_width(&session);
    assert!((width - 100.0).abs() < 1e-6, "the run of eight is the width: {width}");
}

/// A run of **more than `MAX_RUN` characters** is not used.
///
/// The walk looks at eight characters at a time, so a run of nine is
/// never found: the word is measured with the **natural** widths of its
/// characters instead — one millimetre each here.
#[test]
fn a_run_over_the_limit_is_not_used() {
    let run = "abcdefghi";
    assert_eq!(run.chars().count(), MAX_RUN + 1, "the run is one longer than the cap");
    let mut session = session();
    a_paragraph(&mut session, run, json!({ "abcdefghi": 100.0 }));
    let width = the_width(&session);
    let natural = (MAX_RUN + 1) as f64 * 1.0;
    assert!((width - natural).abs() < 1e-6, "the natural width is used: {width}");
    assert!((width - 100.0).abs() > 1e-6, "and the run of nine is not: {width}");
}

/// The measurement says that a run of the metric was **ignored**.
///
/// A run that is too long to be looked for is kept and never used, and
/// the width that is reported is the width of the characters instead. A
/// reader of the measurement is told how many of the metric's runs were
/// left out.
#[test]
fn the_measurement_says_a_run_was_ignored() {
    let mut session = session();
    a_paragraph(&mut session, "abcdefghi", json!({ "abcdefghi": 100.0 }));
    assert_eq!(runs_ignored(&session), 1, "the run of nine was ignored: it is not in the width");
}

/// The measurement says **nothing** when every run of the metric fits.
#[test]
fn the_measurement_says_nothing_when_every_run_fits() {
    let mut session = session();
    a_paragraph(&mut session, "abcdefgh", json!({ "abcdefgh": 100.0 }));
    assert_eq!(runs_ignored(&session), 0, "the run of eight is used: it is in the width");
}
