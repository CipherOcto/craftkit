//! The **flags** of an annotation.

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
fn an_annotation(session: &mut Session, object: u64, flags: Json) -> u64 {
    add_object(
        session,
        object,
        json!([["Type", "Annot"], ["Subtype", "Text"], ["Rect", [0, 0, 10, 10]], ["F", flags],]),
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

/// An annotation that does not **print** breaks the rule.
#[test]
fn an_annotation_that_does_not_print_breaks_the_rule() {
    let mut session = session();
    an_annotation(&mut session, 2, json!(0));
    assert!(clauses_of(&mut session).contains(&"6.3.2".to_string()), "the rule is broken");
}

/// An annotation that is **hidden** breaks the rule.
#[test]
fn an_annotation_that_is_hidden_breaks_the_rule() {
    let mut session = session();
    an_annotation(&mut session, 2, json!(4));
    assert!(!clauses_of(&mut session).contains(&"6.3.2".to_string()), "the rule is not broken");
    an_annotation(&mut session, 3, json!(6));
    assert!(clauses_of(&mut session).contains(&"6.3.2".to_string()), "a hidden one breaks it");
}

/// An annotation that **prints** and is **shown** breaks nothing.
#[test]
fn an_annotation_that_prints_and_is_shown_breaks_nothing() {
    let mut session = session();
    an_annotation(&mut session, 2, json!(4));
    assert!(!clauses_of(&mut session).contains(&"6.3.2".to_string()), "the rule is not broken");
}

/// An annotation may be made to **print**.
#[test]
fn an_annotation_may_be_made_to_print() {
    let mut session = session();
    let annotation = an_annotation(&mut session, 2, json!(0));
    session
        .execute("pdf.annot.flags", &json!({ "annotation": annotation, "print": true }))
        .expect("the flag is said");
    let clauses = clauses_of(&mut session);
    assert!(!clauses.contains(&"6.3.2".to_string()), "the rule is not broken: {clauses:?}");
}

/// An annotation may be **shown** again.
#[test]
fn an_annotation_may_be_shown_again() {
    let mut session = session();
    let annotation = an_annotation(&mut session, 2, json!(6));
    assert!(clauses_of(&mut session).contains(&"6.3.2".to_string()), "the rule is broken");
    session
        .execute("pdf.annot.flags", &json!({ "annotation": annotation, "hidden": false }))
        .expect("the flag is said");
    let clauses = clauses_of(&mut session);
    assert!(!clauses.contains(&"6.3.2".to_string()), "the rule is not broken: {clauses:?}");
}

/// A flag that is **not said** leaves the annotation as it was.
#[test]
fn a_flag_that_is_not_said_leaves_the_annotation_as_it_was() {
    let mut session = session();
    let annotation = an_annotation(&mut session, 2, json!(4));
    session.execute("pdf.annot.flags", &json!({ "annotation": annotation })).expect("nothing is said");
    let clauses = clauses_of(&mut session);
    assert!(!clauses.contains(&"6.3.2".to_string()), "the annotation still prints: {clauses:?}");
}

/// The reading of a page's annotations says the **flags**.
#[test]
fn the_reading_of_a_pages_annotations_says_the_flags() {
    let mut session = session();
    let page = add_object(&mut session, 5, json!([["Type", "Page"]]));
    let outcome = session
        .execute(
            "pdf.annot.add",
            &json!({ "page": page, "object": 2, "subtype": "Text", "rect": [0, 0, 10, 10] }),
        )
        .expect("the annotation is added");
    let annotation = outcome.data["id"].as_u64().expect("the annotation has an id");
    let outcome =
        session.execute("pdf.page.annotations", &json!({ "page": page })).expect("the annotations are read");
    let annotations = outcome.data["annotations"].as_array().expect("the annotations are a list");
    let first = &annotations[0];
    assert_eq!(first["flags"], json!(4), "the flags are said");
    assert_eq!(first["print"], json!(true), "the annotation prints");
    assert_eq!(first["hidden"], json!(false), "and it is shown");
    assert_eq!(first["id"], json!(annotation), "the annotation is the one");
}

/// A node that is **not an annotation** is refused.
#[test]
fn a_node_that_is_not_an_annotation_is_refused() {
    let mut session = session();
    let node = add_object(&mut session, 2, json!([["Type", "XObject"]]));
    let refused = session
        .execute("pdf.annot.flags", &json!({ "annotation": node, "print": true }))
        .expect_err("the node is refused");
    assert!(refused.to_string().contains("is not an annotation"), "the refusal says why: {refused}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let refused = session
        .execute("pdf.annot.flags", &json!({ "annotation": 999, "print": true }))
        .expect_err("the node is refused");
    assert!(refused.to_string().contains("there is no node `999`"), "the refusal says why: {refused}");
}
