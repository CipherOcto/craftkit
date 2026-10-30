//! The prose battery: a word processor's document is a **flat list**, a
//! **side vector** of styles and a table of media.
//!
//! What is proved here: the flat list fits the tree exactly, what is
//! numerous is in the payload and what is few is a node, a style is a
//! node and a paragraph's style is an **edge**, the media is a **stream**,
//! and the blocks are derived.

use craftkit_sdk::Ecosystem;
#[cfg(feature = "prose")]
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A document with a style in it.
#[cfg(feature = "prose")]
fn a_document() -> craftkit_sdk::Session {
    let mut session = ecosystem().session();
    session.execute("prose.document.add", &json!({ "name": "the letter" })).expect("the document is added");
    session
        .execute("prose.style.add", &json!({ "document": 1, "name": "Body" }))
        .expect("the style is added");
    session
}

/// The battery is linked in when the feature is on.
#[cfg(feature = "prose")]
#[test]
fn the_prose_battery_is_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("prose.document.add").is_some(), "the battery is in");
    assert!(ecosystem.kinds.get("prose.block.paragraph").is_some());
    assert!(ecosystem.evaluators.get("prose.blocks").is_some());
}

/// And it is not there when the feature is off.
#[cfg(not(feature = "prose"))]
#[test]
fn the_prose_battery_is_not_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("prose.document.add").is_none(), "the battery is out");
}

/// The block kinds are **open**: a paragraph and a table are two kinds,
/// and nothing limits the list.
#[cfg(feature = "prose")]
#[test]
fn the_block_kinds_are_open() {
    let ecosystem = ecosystem();
    for kind in ["prose.block.paragraph", "prose.block.table"] {
        assert!(ecosystem.kinds.get(kind).is_some(), "{kind} is a kind");
    }
}

/// The **flat list** fits the tree exactly: the blocks are children, and
/// their order is the tree's order.
#[cfg(feature = "prose")]
#[test]
fn the_blocks_are_a_flat_list_in_order() {
    let mut session = a_document();
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "first" }))
        .expect("the paragraph is added");
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "second" }))
        .expect("the paragraph is added");
    session
        .execute("prose.block.table.add", &json!({ "document": 1, "rows": [["a", "b"]] }))
        .expect("the table is added");
    let plan = session.evaluate();
    let blocks: Vec<_> = plan.items.iter().filter(|item| item.kind == "prose.blocks").collect();
    let list = blocks[0].value["blocks"].as_array().expect("a list of blocks");
    assert_eq!(list.len(), 3, "three blocks");
    assert_eq!(list[0]["text"], json!("first"), "in the order they were added");
    assert_eq!(list[1]["text"], json!("second"));
    assert_eq!(list[2]["kind"], json!("prose.block.table"));
}

/// What is **numerous** is in the payload: a paragraph's runs are beside
/// its text, not nodes.
#[cfg(feature = "prose")]
#[test]
fn the_runs_are_in_the_payload() {
    let mut session = a_document();
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "the quick fox" }))
        .expect("the paragraph is added");
    session
        .execute(
            "prose.block.runs",
            &json!({ "block": 3, "runs": [
                { "len": 10, "bold": false },
                { "len": 3, "bold": true }
            ] }),
        )
        .expect("the runs are set");
    let node = session.document().node(3).expect("the paragraph is there");
    assert_eq!(node.data["runs"].as_array().expect("a list").len(), 2, "the runs are in the payload");
    assert_eq!(node.data["text"], json!("the quick fox"), "and the text is beside them");
    // The runs did not become nodes: the document has no new children.
    assert_eq!(session.document().walk().count(), 3, "no node was made for a run");
}

/// A run is an **interval**, and the words it covers are **derived**.
#[cfg(feature = "prose")]
#[test]
fn the_words_a_run_covers_are_derived() {
    let mut session = a_document();
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "the quick fox" }))
        .expect("the paragraph is added");
    session
        .execute(
            "prose.block.runs",
            &json!({ "block": 3, "runs": [
                { "len": 10, "bold": false },
                { "len": 3, "bold": true }
            ] }),
        )
        .expect("the runs are set");
    let plan = session.evaluate();
    let blocks: Vec<_> = plan.items.iter().filter(|item| item.kind == "prose.blocks").collect();
    let runs = blocks[0].value["blocks"][0]["runs"].as_array().expect("a list of runs");
    assert_eq!(runs[0]["text"], json!("the quick "), "the first run covers ten characters");
    assert_eq!(runs[1]["text"], json!("fox"), "and the second covers the rest");
    assert_eq!(runs[1]["bold"], json!(true), "and it says it is bold");
}

/// The words are not stored twice: the run says how many, and the text
/// is stored once.
#[cfg(feature = "prose")]
#[test]
fn the_run_does_not_carry_the_words() {
    let mut session = a_document();
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "the quick fox" }))
        .expect("the paragraph is added");
    session
        .execute("prose.block.runs", &json!({ "block": 3, "runs": [{ "len": 13, "bold": true }] }))
        .expect("the runs are set");
    let node = session.document().node(3).expect("the paragraph is there");
    assert_eq!(node.data["runs"][0]["len"], json!(13), "the run says how many");
    assert!(node.data["runs"][0].get("text").is_none(), "and it does not carry the words");
}

/// Runs that cover more characters than the text has are refused: the
/// input is hostile and the count is bounded by the text.
#[cfg(feature = "prose")]
#[test]
fn runs_that_cover_too_much_are_refused() {
    let mut session = a_document();
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "short" }))
        .expect("the paragraph is added");
    let error = session
        .execute("prose.block.runs", &json!({ "block": 3, "runs": [{ "len": 100 }] }))
        .expect_err("that is too much");
    assert!(error.to_string().contains("more characters than the text has"), "{error}");
}

/// A run that does not say how many it covers is refused.
#[cfg(feature = "prose")]
#[test]
fn a_run_without_a_length_is_refused() {
    let mut session = a_document();
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "short" }))
        .expect("the paragraph is added");
    let error = session
        .execute("prose.block.runs", &json!({ "block": 3, "runs": [{ "bold": true }] }))
        .expect_err("the run says nothing");
    assert!(error.to_string().contains("how many characters"), "{error}");
}

/// A style may be **based on** another style, and that is an edge too.
#[cfg(feature = "prose")]
#[test]
fn a_style_may_be_based_on_another() {
    let mut session = a_document();
    session
        .execute("prose.style.add", &json!({ "document": 1, "name": "Emphasis", "based_on": 2 }))
        .expect("the style is added");
    let edge = session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == "prose.based_on")
        .expect("there is an edge");
    assert_eq!(edge.from, 3, "the edge is from the style");
    assert_eq!(edge.to, 2, "the edge is to the style it is based on");
}

/// And the chain is **derived**: it is walked every time.
#[cfg(feature = "prose")]
#[test]
fn the_style_chain_is_derived() {
    let mut session = a_document();
    session
        .execute("prose.style.add", &json!({ "document": 1, "name": "Emphasis", "based_on": 2 }))
        .expect("the style is added");
    session
        .execute("prose.style.add", &json!({ "document": 1, "name": "Strong", "based_on": 3 }))
        .expect("the style is added");
    let plan = session.evaluate();
    let styles: Vec<_> = plan.items.iter().filter(|item| item.kind == "prose.style").collect();
    let strong = styles.iter().find(|item| item.node == 4).expect("the style is there");
    assert_eq!(strong.value["chain"], json!(["Emphasis", "Body"]), "nearest first");
    assert_eq!(strong.value["based_on"], json!("Emphasis"));
}

/// A cycle of styles does not hang the program: the walk stops.
#[cfg(feature = "prose")]
#[test]
fn a_cycle_of_styles_ends_the_chain() {
    let mut session = a_document();
    session
        .execute("prose.style.add", &json!({ "document": 1, "name": "Emphasis", "based_on": 2 }))
        .expect("the style is added");
    // The style is based on the style it is based on, by hand: the edge
    // is what makes the cycle, not the command.
    let mut document = session.document().clone();
    document.relations.push(craftkit_sdk::Relation::with_payload(2, 3, "prose.based_on", json!({})));
    *session.document_mut() = document;
    let plan = session.evaluate();
    let styles: Vec<_> = plan.items.iter().filter(|item| item.kind == "prose.style").collect();
    let body = styles.iter().find(|item| item.node == 2).expect("the style is there");
    assert_eq!(body.value["chain"], json!(["Emphasis", "Body"]), "the chain ends at the style it saw");
}

/// What is **few** is a node: a style.
#[cfg(feature = "prose")]
#[test]
fn a_style_is_a_node() {
    let session = a_document();
    let style = session.document().node(2).expect("the style is there");
    assert_eq!(style.kind, "prose.style");
    assert_eq!(style.name, "Body");
}

/// A paragraph's style is an **edge**, and not a parent: the paragraph
/// is still in the document.
#[cfg(feature = "prose")]
#[test]
fn a_blocks_style_is_an_edge() {
    let mut session = a_document();
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "first" }))
        .expect("the paragraph is added");
    session.execute("prose.block.style", &json!({ "block": 3, "style": 2 })).expect("it is styled");
    let relations = &session.document().relations;
    let edge =
        relations.iter().find(|relation| relation.kind == "prose.styled_by").expect("there is an edge");
    assert_eq!(edge.from, 3, "the edge is from the block");
    assert_eq!(edge.to, 2, "the edge is to the style");
    assert_eq!(edge.payload["style"], json!("Body"), "and the edge says which style");
    let document = session.document().node(1).expect("the document is there");
    assert_eq!(document.children.len(), 2, "the style and the paragraph are its children");
}

/// And the derived blocks carry the style, because the edge says it.
#[cfg(feature = "prose")]
#[test]
fn the_blocks_carry_the_style_the_edge_says() {
    let mut session = a_document();
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "first" }))
        .expect("the paragraph is added");
    session.execute("prose.block.style", &json!({ "block": 3, "style": 2 })).expect("it is styled");
    let plan = session.evaluate();
    let blocks: Vec<_> = plan.items.iter().filter(|item| item.kind == "prose.blocks").collect();
    assert_eq!(blocks[0].value["blocks"][0]["style"], json!("Body"));
}

/// A block put in a style twice is refused: the edge would be said
/// twice.
#[cfg(feature = "prose")]
#[test]
fn a_block_styled_twice_is_refused() {
    let mut session = a_document();
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "first" }))
        .expect("the paragraph is added");
    session.execute("prose.block.style", &json!({ "block": 3, "style": 2 })).expect("it is styled");
    let error = session
        .execute("prose.block.style", &json!({ "block": 3, "style": 2 }))
        .expect_err("it is already styled");
    assert!(error.to_string().contains("already in the style"), "{error}");
}

/// A style of something that is not a style is refused.
#[cfg(feature = "prose")]
#[test]
fn a_style_of_something_that_is_not_a_style_is_refused() {
    let mut session = a_document();
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "first" }))
        .expect("the paragraph is added");
    let error = session
        .execute("prose.block.style", &json!({ "block": 3, "style": 1 }))
        .expect_err("a document is not a style");
    assert!(error.to_string().contains("not a style"), "{error}");
}

/// The **media** is a stream: the file is the node's stream, and the
/// bytes travel beside the document in the bundle.
#[cfg(feature = "prose")]
#[test]
fn the_media_is_a_stream() {
    let mut session = a_document();
    session
        .execute("prose.media.add", &json!({ "document": 1, "name": "the seal", "path": "media/seal.png" }))
        .expect("the media is added");
    let media = session.document().node(3).expect("the media is there");
    assert_eq!(media.kind, "prose.media");
    assert_eq!(media.stream.as_deref(), Some("media/seal.png"), "the file is the stream");
}

/// The runs are **not** a list is refused.
#[cfg(feature = "prose")]
#[test]
fn runs_that_are_not_a_list_are_refused() {
    let mut session = a_document();
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "first" }))
        .expect("the paragraph is added");
    let error = session
        .execute("prose.block.runs", &json!({ "block": 3, "runs": "bold" }))
        .expect_err("the runs are not a list");
    assert!(error.to_string().contains("the runs are a list"), "{error}");
}

/// A hundred thousand runs is refused: the input is hostile and the
/// count is capped.
#[cfg(feature = "prose")]
#[test]
fn a_hundred_thousand_runs_are_refused() {
    let mut session = a_document();
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "first" }))
        .expect("the paragraph is added");
    let runs: Vec<_> = (0..100_001).map(|_| json!({ "len": 1 })).collect();
    let error = session
        .execute("prose.block.runs", &json!({ "block": 3, "runs": runs }))
        .expect_err("that is too many runs");
    assert!(error.to_string().contains("hundred thousand"), "{error}");
}

/// The rows of a table are a list, and a table is a **payload**.
#[cfg(feature = "prose")]
#[test]
fn the_rows_are_in_the_payload() {
    let mut session = a_document();
    session
        .execute("prose.block.table.add", &json!({ "document": 1, "rows": [["a", "b"], ["c", "d"]] }))
        .expect("the table is added");
    let node = session.document().node(3).expect("the table is there");
    assert_eq!(node.kind, "prose.block.table");
    assert_eq!(node.data["rows"].as_array().expect("a list").len(), 2, "the rows are in the payload");
}

/// Rows that are not a list are refused.
#[cfg(feature = "prose")]
#[test]
fn rows_that_are_not_a_list_are_refused() {
    let mut session = a_document();
    let error = session
        .execute("prose.block.table.add", &json!({ "document": 1, "rows": "a table" }))
        .expect_err("the rows are not a list");
    assert!(error.to_string().contains("the rows are a list"), "{error}");
}

/// A paragraph in something that is not a document is refused.
#[cfg(feature = "prose")]
#[test]
fn a_paragraph_in_something_that_is_not_a_document_is_refused() {
    let mut session = a_document();
    let error = session
        .execute("prose.block.paragraph.add", &json!({ "document": 2, "text": "first" }))
        .expect_err("a style is not a document");
    assert!(error.to_string().contains("not a document"), "{error}");
}

/// A document with no name is refused.
#[cfg(feature = "prose")]
#[test]
fn a_document_with_no_name_is_refused() {
    let mut session = ecosystem().session();
    let error =
        session.execute("prose.document.add", &json!({ "name": "  " })).expect_err("a document has a name");
    assert!(error.to_string().contains("a document has a name"), "{error}");
}

/// The undo takes the edge with it.
#[cfg(feature = "prose")]
#[test]
fn the_undo_takes_the_edge_with_it() {
    let mut session = a_document();
    session
        .execute("prose.block.paragraph.add", &json!({ "document": 1, "text": "first" }))
        .expect("the paragraph is added");
    session.execute("prose.block.style", &json!({ "block": 3, "style": 2 })).expect("it is styled");
    assert_eq!(session.document().relations.len(), 1);
    session.undo().expect("the undo works");
    assert_eq!(session.document().relations.len(), 0, "the edge is gone");
    session.undo().expect("the undo works again");
    assert_eq!(session.document().walk().count(), 2, "and so is the paragraph");
}
