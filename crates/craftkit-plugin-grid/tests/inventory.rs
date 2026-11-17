//! The **list** of what the battery knows.

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

/// A **workbook** with a sheet in it, and their ids.
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

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The battery's **list** of what it knows.
fn inventory() -> Vec<(&'static str, &'static str)> {
    craftkit_plugin_grid::inventory::functions()
}

/// **No name is in two families.** A name that two families carry is a
/// name the order of the dispatch decides, and a list that says
/// otherwise is a list that lies.
#[test]
fn no_name_is_in_two_families() {
    let mut seen: Vec<&str> = Vec::new();
    for (name, _) in inventory() {
        assert!(!seen.contains(&name), "the name {name} is in two families");
        seen.push(name);
    }
}

/// **The list is the tree's list, and eleven more.** The tree's
/// registry holds four hundred and ninety functions, and this battery
/// knows every one of them — plus `TEXT` and `SUBTOTAL`, which the
/// tree's own functions crate leaves to its host, `EXPON`, this tree's
/// own alias of `EXPONDIST`, and the twenty-two **host** names of
/// §896, §897 and §898.
#[test]
fn the_list_is_the_trees_list() {
    assert_eq!(inventory().len(), 515, "the number of functions the battery knows");
}

/// **Every name in the list is answered.** A name in the list that the
/// engine does not know is a name that says it can do something it
/// cannot.
#[test]
fn every_name_in_the_list_is_answered() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let names: Vec<&str> = inventory().into_iter().map(|(name, _)| name).collect();
    let mut unanswered: Vec<&str> = Vec::new();
    for (index, name) in names.iter().enumerate() {
        // A cell far down the sheet, so that the formulas do not sit
        // on top of one another.
        let cell = format!("A{}", index + 1);
        formula(&mut session, sheet, &cell, &format!("={name}()"));
        let value = value_of(&mut session, sheet, &cell);
        let message = value["error"].as_str().unwrap_or_default();
        if message.contains("there is no function") {
            unanswered.push(name);
        }
    }
    assert!(unanswered.is_empty(), "the engine does not answer {:?}", unanswered);
}

/// **A name the battery does not know is not in the list.**
#[test]
fn a_name_the_battery_does_not_know_is_not_in_the_list() {
    for name in ["NOTAFUNCTION", "ABSOLUTELYNOT", "ZZZ"] {
        assert!(!craftkit_plugin_grid::inventory::knows(name), "{name} is not a function");
        assert!(craftkit_plugin_grid::inventory::family_of(name).is_none(), "{name} has no family");
    }
}

/// **The list says which family carries a name.**
#[test]
fn the_list_says_which_family_carries_a_name() {
    use craftkit_plugin_grid::inventory::Family;
    assert_eq!(craftkit_plugin_grid::inventory::family_of("ABS"), Some(Family::Arithmetic));
    assert_eq!(craftkit_plugin_grid::inventory::family_of("SUM"), Some(Family::Core));
    assert_eq!(craftkit_plugin_grid::inventory::family_of("NOW"), Some(Family::Outside));
    assert_eq!(craftkit_plugin_grid::inventory::family_of("CUBESET"), Some(Family::Refused));
    assert_eq!(craftkit_plugin_grid::inventory::family_of("MMULT"), Some(Family::Array));
    assert_eq!(craftkit_plugin_grid::inventory::family_of("FORECAST.ETS"), Some(Family::Ets));
}

/// **The registry and the list agree.** The registry says which family
/// carries a name, and the list says the same, and the two are one
/// thing read two ways.
#[test]
fn the_registry_and_the_list_agree() {
    for (name, family) in inventory() {
        let found = craftkit_plugin_grid::inventory::family_of(name);
        assert!(found.is_some(), "{name} is in the list and in no family");
        assert_eq!(found.map(|family| family.name()), Some(family), "the family of {name}");
    }
}

/// **The tree's booleans are functions as well as values.**
#[test]
fn the_trees_booleans_are_functions_as_well_as_values() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "D1", "=TRUE()");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(true), "the function TRUE");
    formula(&mut session, sheet, "D2", "=FALSE()");
    assert_eq!(value_of(&mut session, sheet, "D2"), json!(false), "the function FALSE");
    formula(&mut session, sheet, "D3", "=TRUE");
    assert_eq!(value_of(&mut session, sheet, "D3"), json!(true), "the value TRUE");
    formula(&mut session, sheet, "D4", "=NOT(TRUE())");
    assert_eq!(value_of(&mut session, sheet, "D4"), json!(false), "the function inside another");
    formula(&mut session, sheet, "D5", "=TRUE(1)");
    assert_eq!(
        value_of(&mut session, sheet, "D5")["error"],
        Json::String("#VALUE!".to_string()),
        "the tree's booleans take no arguments"
    );
}

/// **The functions of the tree may be listed.** The answer is the
/// registry itself, with the family of each name.
#[test]
fn the_functions_of_the_tree_may_be_listed() {
    let mut session = session();
    let outcome = session.execute("grid.formula.functions", &json!({})).expect("the functions are listed");
    let functions = outcome.data["functions"].as_array().expect("the functions are a list");
    assert_eq!(functions.len(), inventory().len(), "every function is in the list");
    let sum = functions.iter().find(|function| function["name"] == json!("SUM")).expect("SUM is there");
    assert_eq!(sum["family"], json!("core"), "and the family of it is said");
    assert!(sum.get("reason").is_none(), "and a function that is here has no reason to be elsewhere");
    // A function that is **refused** is in the list with the **reason**
    // it is not here: a function that is refused with a reason is not
    // missing, it is answered.
    let offset =
        functions.iter().find(|function| function["name"] == json!("OFFSET")).expect("OFFSET is there");
    assert_eq!(offset["family"], json!("host"), "the family of it is said");
    assert_eq!(
        offset["reason"],
        json!("not supported yet: a reference that is worked out"),
        "and the reason it is not here is said"
    );
}

/// The list may be **searched** and **kept to one family**.
#[test]
fn the_list_may_be_searched() {
    let mut session = session();
    let outcome = session
        .execute("grid.formula.functions", &json!({ "search": "count" }))
        .expect("the functions are listed");
    let functions = outcome.data["functions"].as_array().expect("the functions are a list");
    assert!(!functions.is_empty(), "some names hold the text");
    for function in functions {
        assert!(
            function["name"].as_str().unwrap_or_default().to_lowercase().contains("count"),
            "the name holds the text: {function}"
        );
    }
    let outcome = session
        .execute("grid.formula.functions", &json!({ "category": "arithmetic" }))
        .expect("the functions are listed");
    let functions = outcome.data["functions"].as_array().expect("the functions are a list");
    assert!(!functions.is_empty(), "the family has functions");
    for function in functions {
        assert_eq!(function["family"], json!("arithmetic"), "the family is the one that was asked for");
    }
}
