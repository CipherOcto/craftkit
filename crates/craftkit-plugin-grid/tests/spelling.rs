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

/// A word the user **taught** the workbook is a word.
#[test]
fn a_word_the_user_taught_is_a_word() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("teh cat"));
    let list = a_word_list(&["cat"]);
    // Without the teaching, the word is an issue.
    let found = issues(&mut session, sheet, &list);
    assert_eq!(found.as_array().map(|issues| issues.len()), Some(1), "the word is an issue: {found}");
    session
        .execute("grid.spelling.add", &json!({ "workbook": workbook, "word": "teh" }))
        .expect("the workbook learns the word");
    // And with it, it is not.
    let found = issues(&mut session, sheet, &list);
    assert_eq!(found.as_array().map(|issues| issues.len()), Some(0), "the word is not an issue: {found}");
}

/// A word the workbook already knows is not taught twice.
#[test]
fn a_word_the_workbook_knows_is_not_taught_twice() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session);
    session
        .execute("grid.spelling.add", &json!({ "workbook": workbook, "word": "teh" }))
        .expect("the workbook learns the word");
    let outcome = session
        .execute("grid.spelling.add", &json!({ "workbook": workbook, "word": "TEH" }))
        .expect("the word is taught again");
    assert_eq!(outcome.data["words"].as_u64(), Some(1), "the word is taught once: {}", outcome.data);
}

/// The **thesaurus**: the words that share a stem with a word.
#[test]
fn the_thesaurus_gives_the_words_that_share_a_stem() {
    let mut session = session();
    let (_workbook, _sheet) = a_sheet(&mut session);
    let list = a_word_list(&["cat", "cats", "cattle", "dog", "caterpillar"]);
    let outcome = session
        .execute("grid.thesaurus", &json!({ "word": "cat", "dictionary": list }))
        .expect("the thesaurus is asked");
    let related = outcome.data["related"].as_array().expect("the words are a list");
    // The stem of `cat` is `cat`, and the words that begin with it and
    // are no longer than seven letters are related.
    let names: Vec<&str> = related.iter().filter_map(|word| word.as_str()).collect();
    assert!(names.contains(&"cats"), "the words that share the stem are there: {names:?}");
    assert!(names.contains(&"cattle"), "the words that share the stem are there: {names:?}");
    assert!(!names.contains(&"caterpillar"), "a word that is too long is not related: {names:?}");
    assert!(!names.contains(&"dog"), "a word that does not share the stem is not related: {names:?}");
}
