//! The **equation** of a document: the parts and the text derived from
//! them.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_prose::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the prose battery.
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

/// A **document**.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    outcome.data["id"].as_u64().expect("the document has an id")
}

/// An **equation** in a document.
fn an_equation(session: &mut Session, document: u64, parts: Json) -> u64 {
    let outcome = session
        .execute("prose.block.equation.add", &json!({ "document": document, "parts": parts }))
        .expect("the equation is added");
    outcome.data["id"].as_u64().expect("the equation has an id")
}

/// The **text** of a block.
fn text_of(session: &Session, block: u64) -> String {
    let node = session.document().node(block).expect("the block is in the document");
    node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string()
}

/// A **text** part.
fn text(part: &str) -> Json {
    json!({ "kind": "text", "text": part })
}

/// **The text of an equation is derived from its parts.**
#[test]
fn the_text_of_an_equation_is_derived() {
    let mut session = session();
    let document = a_document(&mut session);
    let equation = an_equation(
        &mut session,
        document,
        json!({ "kind": "fraction", "top": text("a"), "bottom": text("b") }),
    );
    assert_eq!(text_of(&session, equation), "a/b");
}

/// **Every part of an equation is written on one line.**
#[test]
fn every_part_is_written_on_one_line() {
    let mut session = session();
    let document = a_document(&mut session);
    let equation = an_equation(
        &mut session,
        document,
        json!({
            "kind": "superscript",
            "base": text("x"),
            "above": text("2"),
        }),
    );
    assert_eq!(text_of(&session, equation), "x^2");
    let equation = an_equation(
        &mut session,
        document,
        json!({ "kind": "subscript", "base": text("x"), "below": text("1") }),
    );
    assert_eq!(text_of(&session, equation), "x_1");
    let equation = an_equation(&mut session, document, json!({ "kind": "radical", "inside": text("x") }));
    assert_eq!(text_of(&session, equation), "√x");
}

/// **A part that is put inside another is written in brackets.**
#[test]
fn a_nested_part_is_written_in_brackets() {
    let mut session = session();
    let document = a_document(&mut session);
    let equation = an_equation(
        &mut session,
        document,
        json!({
            "kind": "fraction",
            "top": json!({ "kind": "superscript", "base": text("x"), "above": text("2") }),
            "bottom": text("y"),
        }),
    );
    assert_eq!(text_of(&session, equation), "(x^2)/y");
}

/// **A part that is not one of the five is refused.**
#[test]
fn a_part_that_is_not_one_of_the_five_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let failure = session
        .execute(
            "prose.block.equation.add",
            &json!({ "document": document, "parts": json!({ "kind": "integral" }) }),
        )
        .expect_err("the equation is refused");
    let message = failure.to_string();
    assert!(message.contains("is not a part of an equation"), "the refusal says why: {message}");
}

/// **A part that is missing a piece is refused.**
#[test]
fn a_part_that_is_missing_a_piece_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let failure = session
        .execute(
            "prose.block.equation.add",
            &json!({ "document": document, "parts": json!({ "kind": "fraction", "top": text("a") }) }),
        )
        .expect_err("the equation is refused");
    let message = failure.to_string();
    assert!(message.contains("has no `bottom`"), "the refusal says why: {message}");
}

/// **A text that holds no text is refused.**
#[test]
fn a_text_that_holds_no_text_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let failure = session
        .execute(
            "prose.block.equation.add",
            &json!({ "document": document, "parts": json!({ "kind": "text", "text": 7 }) }),
        )
        .expect_err("the equation is refused");
    let message = failure.to_string();
    assert!(message.contains("holds text"), "the refusal says why: {message}");
}

/// **The parts of an equation may be set again.**
#[test]
fn the_parts_may_be_set_again() {
    let mut session = session();
    let document = a_document(&mut session);
    let equation = an_equation(&mut session, document, text("x"));
    session
        .execute(
            "prose.equation.set",
            &json!({
                "block": equation,
                "parts": json!({ "kind": "superscript", "base": text("x"), "above": text("3") }),
            }),
        )
        .expect("the parts are set");
    assert_eq!(text_of(&session, equation), "x^3");
}

/// **A block that is not an equation is refused.**
#[test]
fn a_block_that_is_not_an_equation_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "the words" }))
        .expect("the paragraph is added");
    let paragraph = outcome.data["id"].as_u64().expect("the paragraph has an id");
    let failure = session
        .execute("prose.equation.set", &json!({ "block": paragraph, "parts": text("x") }))
        .expect_err("the parts are refused");
    let message = failure.to_string();
    assert!(message.contains("and not an equation"), "the refusal says why: {message}");
}

/// **The text of an equation is not replaced.**
#[test]
fn the_text_of_an_equation_is_not_replaced() {
    let mut session = session();
    let document = a_document(&mut session);
    let equation = an_equation(&mut session, document, text("hello"));
    let failure = session
        .execute("prose.replace", &json!({ "block": equation, "what": "hello", "with": "goodbye" }))
        .expect_err("the replace is refused");
    let message = failure.to_string();
    assert!(message.contains("derived from its parts"), "the refusal says why: {message}");
    assert_eq!(text_of(&session, equation), "hello", "and the equation is as it was");
}

/// **The parts of an equation are read back with its text.**
#[test]
fn the_parts_are_read_back() {
    let mut session = session();
    let document = a_document(&mut session);
    let equation = an_equation(&mut session, document, text("x"));
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == equation).expect("the equation is in the plan");
    assert_eq!(item.value["text"], json!("x"));
    assert_eq!(item.value["parts"]["kind"], json!("text"));
}
