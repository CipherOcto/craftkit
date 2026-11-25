//! The **limits of the grid battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. The text that comes in is from
//! outside — a file, a paste, an agent — so it is checked before it is
//! used, and a limit that no test names is a limit nothing proves holds.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_grid::CRAFTKIT_PLUGIN_ANCHOR;

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

/// A **workbook**, and its id.
fn a_workbook(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    outcome.data["id"].as_u64().expect("the workbook has an id")
}

/// A text with **more rows** than `MAX_ROWS` allows is refused.
///
/// The text of an import comes from outside, and a text of ten thousand
/// and one rows is refused rather than half-read.
#[test]
fn a_text_with_too_many_rows_is_refused() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let text = "a\n".repeat(10_001);
    let error = session
        .execute("grid.import.csv", &json!({ "workbook": workbook, "text": text }))
        .expect_err("that text has too many rows");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A text with **more columns** than `MAX_COLUMNS` allows is refused.
#[test]
fn a_text_with_too_many_columns_is_refused() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let text = format!("a{}", ",a".repeat(257));
    let error = session
        .execute("grid.import.csv", &json!({ "workbook": workbook, "text": text }))
        .expect_err("that text has too many columns");
    assert!(error.to_string().contains("at most"), "{error}");
}
