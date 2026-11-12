//! The **annotations** of a converted file.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the pdf battery.
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

/// The **node** of an object, by its number.
fn node_of(session: &Session, object: u64) -> u64 {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|number| number.as_u64()) == Some(object))
        .map(|node| node.id)
        .expect("the object is there")
}

/// An **object** with a dictionary.
fn add_object(session: &mut Session, object: u64, entries: Json) -> u64 {
    session
        .execute("pdf.object.add", &json!({ "object": object, "entries": entries }))
        .expect("the object is added");
    node_of(session, object)
}

/// An **annotation** with the given flags.
fn an_annotation(session: &mut Session, object: u64, subtype: &str, flags: Json) -> u64 {
    add_object(
        session,
        object,
        json!([["Type", "Annot"], ["Subtype", subtype], ["Rect", [0, 0, 10, 10]], ["F", flags],]),
    )
}

/// The **clauses** the checker reports.
fn clauses_of(session: &mut Session) -> Vec<String> {
    let outcome = session.execute("pdf.preflight", &json!({})).expect("the checker runs");
    outcome.data["issues"]
        .as_array()
        .expect("the issues are a list")
        .iter()
        .filter_map(|issue| issue["clause"].as_str().map(|clause| clause.to_string()))
        .collect()
}

/// The **flags** of an annotation, as the document holds them.
fn flags_of(session: &Session, object: u64) -> i64 {
    let node = session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|number| number.as_u64()) == Some(object))
        .expect("the annotation is there");
    let entries = node.data["entries"].clone();
    entries
        .as_array()
        .expect("the entries are a list")
        .iter()
        .find_map(|entry| {
            let pair = entry.as_array()?;
            if pair.first()?.as_str()? != "F" {
                return None;
            }
            Some(pair.get(1)?.as_i64().unwrap_or_default())
        })
        .unwrap_or_default()
}

/// The conversion makes an annotation **print** and **shown**.
#[test]
fn the_conversion_makes_an_annotation_print_and_shown() {
    let mut session = session();
    an_annotation(&mut session, 2, "Text", json!(0));
    assert!(clauses_of(&mut session).contains(&"6.3.2".to_string()), "the rule is broken");
    session.execute("pdf.convert", &json!({})).expect("the file is converted");
    assert_eq!(flags_of(&session, 2), 4, "the annotation prints");
    let clauses = clauses_of(&mut session);
    assert!(!clauses.contains(&"6.3.2".to_string()), "the rule is not broken: {clauses:?}");
}

/// The conversion **shows** an annotation that is hidden.
#[test]
fn the_conversion_shows_an_annotation_that_is_hidden() {
    let mut session = session();
    an_annotation(&mut session, 2, "Text", json!(6));
    session.execute("pdf.convert", &json!({})).expect("the file is converted");
    assert_eq!(flags_of(&session, 2), 4, "the annotation prints and is shown");
}

/// A **popup** is left alone.
#[test]
fn a_popup_is_left_alone() {
    let mut session = session();
    an_annotation(&mut session, 2, "Popup", json!(0));
    session.execute("pdf.convert", &json!({})).expect("the file is converted");
    assert_eq!(flags_of(&session, 2), 0, "the popup is as it was");
}

/// The conversion says how many annotations it **changed**.
#[test]
fn the_conversion_says_how_many_annotations_it_changed() {
    let mut session = session();
    an_annotation(&mut session, 2, "Text", json!(0));
    an_annotation(&mut session, 3, "Text", json!(4));
    let outcome = session.execute("pdf.convert", &json!({})).expect("the file is converted");
    let fixed = outcome.data["fixed"].as_array().expect("the fixes are a list");
    let said = fixed.iter().any(|entry| entry.as_str().unwrap_or_default().contains("print and show"));
    assert!(said, "the conversion says so: {fixed:?}");
}
