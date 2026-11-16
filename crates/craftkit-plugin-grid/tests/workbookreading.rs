//! **Reading** a workbook.

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

/// A **workbook** with a **sheet** in it.
fn a_workbook(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// The **answer** of a read.
fn read(session: &mut Session, workbook: u64) -> Json {
    let outcome =
        session.execute("grid.workbook.get", &json!({ "workbook": workbook })).expect("the workbook is read");
    outcome.data.clone()
}

/// A workbook may be **read whole**.
#[test]
fn a_workbook_may_be_read_whole() {
    let mut session = session();
    let (workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 3 }))
        .expect("the cell is set");
    let answer = read(&mut session, workbook);
    assert_eq!(answer["id"], json!(workbook), "the workbook is named by its id");
    assert_eq!(answer["name"], Json::String("the book".to_string()), "and by its name");
    assert_eq!(answer["date_system"], Json::String("1900".to_string()), "and by its calendar");
    assert_eq!(answer["sheets"][0]["id"], json!(sheet), "and its sheet is said");
    assert_eq!(answer["sheets"][0]["name"], Json::String("Sheet1".to_string()), "and the name of it");
    assert_eq!(answer["sheets"][0]["used_range"], Json::String("A1".to_string()), "and what it holds");
    assert_eq!(answer["sheets"][0]["cells"], json!(1), "and how many cells hold something");
}

/// The **names** and the **styles** of a workbook are said by the
/// answer.
#[test]
fn the_names_and_the_styles_are_said_by_the_answer() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    session
        .execute("grid.name.set", &json!({ "workbook": workbook, "name": "Sales", "refers_to": "Sheet1!A1" }))
        .expect("the name is written");
    session
        .execute("grid.style.add", &json!({ "workbook": workbook, "name": "money", "format": "0.00" }))
        .expect("the style is added");
    let answer = read(&mut session, workbook);
    assert_eq!(answer["names"]["Sales"], Json::String("Sheet1!A1".to_string()), "the name is said");
    assert_eq!(answer["styles"][0]["name"], Json::String("money".to_string()), "and the style");
    assert_eq!(answer["styles"][0]["format"], Json::String("0.00".to_string()), "and the format it carries");
}

/// A workbook that is **not there** is refused, and so is a node that
/// is not a workbook.
#[test]
fn a_workbook_that_is_not_there_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.workbook.get", &json!({ "workbook": 999 }))
        .expect_err("the workbook is refused");
    assert!(error.to_string().contains("there is no workbook `999`"), "the refusal says why: {error}");
    let error = session
        .execute("grid.workbook.get", &json!({ "workbook": sheet }))
        .expect_err("the sheet is not a workbook");
    assert!(error.to_string().contains("not a workbook"), "the refusal says why: {error}");
}

/// The **properties** of a workbook may be **set**, and what is set is
/// said by the answer.
#[test]
fn the_properties_of_a_workbook_may_be_set() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let outcome = session
        .execute(
            "grid.workbook.properties.set",
            &json!({
                "workbook": workbook,
                "title": "The sales of the year",
                "subject": "Sales",
                "author": "A. Writer",
                "keywords": "sales year",
                "description": "The numbers",
                "company": "The Company Ltd",
            }),
        )
        .expect("the properties are set");
    assert_eq!(outcome.data["properties"]["title"], json!("The sales of the year"), "the title is said");
    let answer = read(&mut session, workbook);
    assert_eq!(answer["properties"]["author"], json!("A. Writer"), "the author is said");
    assert_eq!(answer["properties"]["company"], json!("The Company Ltd"), "the company is said");
    assert_eq!(answer["properties"]["last_modified_by"], Json::Null, "and what was not said is nothing");
}

/// A property that is **not given** keeps what it was, and a property
/// given as an **empty text** is taken away.
#[test]
fn a_property_that_is_not_given_keeps_what_it_was() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    session
        .execute(
            "grid.workbook.properties.set",
            &json!({ "workbook": workbook, "title": "A title", "subject": "A subject" }),
        )
        .expect("the properties are set");
    session
        .execute("grid.workbook.properties.set", &json!({ "workbook": workbook, "subject": "" }))
        .expect("the properties are set");
    let answer = read(&mut session, workbook);
    assert_eq!(answer["properties"]["title"], json!("A title"), "the title kept what it was");
    assert_eq!(answer["properties"]["subject"], Json::Null, "the subject is taken away");
}

/// The **theme** of a workbook may be read, and a workbook that has no
/// theme says so: the colours are nothing, and nothing is said.
#[test]
fn the_theme_of_a_workbook_may_be_read() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let outcome =
        session.execute("grid.workbook.theme", &json!({ "workbook": workbook })).expect("the theme is read");
    assert_eq!(outcome.data["theme"]["name"], Json::Null, "there is no theme");
    assert_eq!(outcome.data["theme"]["colours"], Json::Null, "and no colours");
    let answer = read(&mut session, workbook);
    assert_eq!(answer["theme"]["name"], Json::Null, "and the whole answer says so too");
}

/// A node that is not a workbook is refused, and so is a workbook that
/// is not there.
#[test]
fn the_theme_of_a_node_that_is_not_a_workbook_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.workbook.theme", &json!({ "workbook": 999 }))
        .expect_err("the workbook is refused");
    assert!(error.to_string().contains("there is no workbook `999`"), "the refusal says why: {error}");
    let error = session
        .execute("grid.workbook.theme", &json!({ "workbook": sheet }))
        .expect_err("the sheet is not a workbook");
    assert!(error.to_string().contains("not a workbook"), "the refusal says why: {error}");
}
