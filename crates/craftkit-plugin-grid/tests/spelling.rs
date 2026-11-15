//! The **spelling** of a sheet.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_grid::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the grid battery.
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

/// A **workbook** with a **sheet** in it, and their ids.
fn a_sheet(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// A **word list** of those words, one a line.
///
/// The list is written here and not shipped: the tree carries no
/// artwork and no word list either.
fn a_word_list(words: &[&str]) -> String {
    let path = craftkit_scratch::file("craftkit-v2-grid-words.txt").expect("the scratch gives a path");
    std::fs::write(&path, words.join("\n")).expect("the word list is written");
    path.to_string_lossy().to_string()
}

/// The **issues** a check finds.
fn issues(session: &mut Session, sheet: u64, dictionary: &str) -> Json {
    let outcome = session
        .execute("grid.spelling.check", &json!({ "sheet": sheet, "dictionary": dictionary }))
        .expect("the sheet is checked");
    outcome.data["issues"].clone()
}

/// A word that is not a word is an **issue**, with a suggestion.
#[test]
fn a_misspelled_word_is_an_issue() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("the teh cat"));
    let list = a_word_list(&["the", "cat", "sat", "mat"]);
    let found = issues(&mut session, sheet, &list);
    let issues = found.as_array().expect("the issues are a list");
    assert_eq!(issues.len(), 1, "one word is not a word: {found}");
    assert_eq!(issues[0]["word"].as_str(), Some("teh"), "the word is named");
    assert_eq!(issues[0]["cell"].as_str(), Some("A1"), "the cell is named");
    let suggestions = issues[0]["suggestions"].as_array().expect("the suggestions are a list");
    assert!(
        suggestions.iter().any(|word| word.as_str() == Some("the")),
        "the word is suggested: {suggestions:?}"
    );
}

/// A word the list holds is not an issue.
#[test]
fn a_word_of_the_list_is_not_an_issue() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("the cat sat"));
    let list = a_word_list(&["the", "cat", "sat", "mat"]);
    let found = issues(&mut session, sheet, &list);
    assert_eq!(found.as_array().map(|issues| issues.len()), Some(0), "no word is an issue: {found}");
}

/// An **inflection** of a word the list holds is not an issue.
#[test]
fn an_inflection_is_not_an_issue() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("cats mating faster"));
    value(&mut session, sheet, "A2", json!("stories"));
    let list = a_word_list(&["cat", "mat", "fast", "story"]);
    let found = issues(&mut session, sheet, &list);
    assert_eq!(found.as_array().map(|issues| issues.len()), Some(0), "no word is an issue: {found}");
}

/// A word that is **not** a word at all — a number, a single letter, an
/// acronym — is not an issue.
#[test]
fn a_word_that_is_not_a_word_is_not_an_issue() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("NASA 42 x"));
    let list = a_word_list(&["the", "cat"]);
    let found = issues(&mut session, sheet, &list);
    assert_eq!(found.as_array().map(|issues| issues.len()), Some(0), "no word is an issue: {found}");
}

/// A **cell with a formula** is not checked: its text is not written
/// down.
#[test]
fn a_cell_with_a_formula_is_not_checked() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=\"teh\"" }))
        .expect("the formula is set");
    let list = a_word_list(&["the", "cat"]);
    let found = issues(&mut session, sheet, &list);
    assert_eq!(found.as_array().map(|issues| issues.len()), Some(0), "no word is an issue: {found}");
}

/// A word list that **cannot be read** is refused.
#[test]
fn a_word_list_that_cannot_be_read_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("teh"));
    let outcome = session
        .execute("grid.spelling.check", &json!({ "sheet": sheet, "dictionary": "/nowhere/at/all.txt" }))
        .expect_err("a word list that cannot be read is refused");
    let message = outcome.to_string();
    assert!(message.contains("could not be read"), "the refusal says why: {message}");
}
