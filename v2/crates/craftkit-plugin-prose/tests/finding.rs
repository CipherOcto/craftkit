//! **Finding** and **replacing** in a block.

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

/// A **paragraph** with a text and its runs.
fn a_paragraph(session: &mut Session, text: &str, runs: Json) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": text }))
        .expect("the paragraph is added");
    let block = outcome.data["id"].as_u64().expect("the paragraph has an id");
    session.execute("prose.block.runs", &json!({ "block": block, "runs": runs })).expect("the runs are set");
    block
}

/// The **text** of a block.
fn text_of(session: &mut Session, block: u64) -> String {
    let node = session.document().node(block).expect("the block is in the document");
    node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string()
}

/// The **runs** of a block.
fn runs_of(session: &mut Session, block: u64) -> Vec<Json> {
    let node = session.document().node(block).expect("the block is in the document");
    node.data.get("runs").and_then(|runs| runs.as_array()).cloned().expect("the runs are a list")
}

/// A **find** says where the pattern is, and changes nothing.
#[test]
fn a_find_says_where_the_pattern_is() {
    let mut session = session();
    let block = a_paragraph(&mut session, "the quick brown fox", json!([{ "len": 19 }]));
    let outcome = session
        .execute("prose.find", &json!({ "block": block, "what": "quick" }))
        .expect("the text is searched");
    let matches = outcome.data["matches"].as_array().cloned().expect("the matches are a list");
    assert_eq!(matches.len(), 1, "there is one match");
    assert_eq!(matches[0]["start"], json!(4), "it begins at the fourth character");
    assert_eq!(matches[0]["end"], json!(9), "and ends at the ninth");
    assert_eq!(matches[0]["text"], json!("quick"), "and it is the word that was found");
    assert_eq!(text_of(&mut session, block), "the quick brown fox", "and the text is as it was");
}

/// A find does not mind the **case**, unless it is told to.
#[test]
fn a_find_does_not_mind_the_case() {
    let mut session = session();
    let block = a_paragraph(&mut session, "the quick brown fox", json!([{ "len": 19 }]));
    let outcome = session
        .execute("prose.find", &json!({ "block": block, "what": "QUICK" }))
        .expect("the text is searched");
    assert_eq!(outcome.data["matches"].as_array().map(|found| found.len()), Some(1), "it is found");
    let outcome = session
        .execute("prose.find", &json!({ "block": block, "what": "QUICK", "case": true }))
        .expect("the text is searched");
    assert_eq!(
        outcome.data["matches"].as_array().map(|found| found.len()),
        Some(0),
        "and it is not found when the case matters"
    );
}

/// A **wildcard** stands for any text, and a `?` for one character.
#[test]
fn a_wildcard_stands_for_any_text() {
    let mut session = session();
    let block = a_paragraph(&mut session, "the quick brown fox", json!([{ "len": 19 }]));
    let outcome = session
        .execute("prose.find", &json!({ "block": block, "what": "quick*fox", "wildcards": true }))
        .expect("the text is searched");
    let matches = outcome.data["matches"].as_array().cloned().expect("the matches are a list");
    assert_eq!(matches.len(), 1, "there is one match");
    assert_eq!(matches[0]["text"], json!("quick brown fox"), "and it is the words between");
}

/// A **replace** rewrites the text and the runs together.
#[test]
fn a_replace_rewrites_the_text_and_the_runs() {
    let mut session = session();
    let block =
        a_paragraph(&mut session, "the quick brown fox", json!([{ "len": 4 }, { "len": 15, "bold": true }]));
    session
        .execute("prose.replace", &json!({ "block": block, "what": "quick", "with": "slow" }))
        .expect("the text is replaced");
    assert_eq!(text_of(&mut session, block), "the slow brown fox", "the text is rewritten");
    let runs = runs_of(&mut session, block);
    assert_eq!(runs.len(), 2, "and there are still two runs");
    assert_eq!(runs[0]["len"], json!(4), "the first run is four characters long");
    assert_eq!(runs[1]["len"], json!(14), "and the second is fourteen");
    assert_eq!(runs[1]["bold"], json!(true), "and it is still bold");
}

/// The replaced words carry the formatting of the run the match
/// **begins** in.
#[test]
fn the_replaced_words_carry_the_formatting_of_the_run() {
    let mut session = session();
    let block = a_paragraph(
        &mut session,
        "the quick brown fox",
        json!([{ "len": 4 }, { "len": 6, "bold": true }, { "len": 9 }]),
    );
    session
        .execute("prose.replace", &json!({ "block": block, "what": "quick", "with": "slow" }))
        .expect("the text is replaced");
    assert_eq!(text_of(&mut session, block), "the slow brown fox", "the text is rewritten");
    let runs = runs_of(&mut session, block);
    assert_eq!(runs[1]["bold"], json!(true), "the run of the replaced words is bold");
    assert_eq!(
        runs[1]["len"],
        json!(5),
        "and it is five characters long: the replaced words and the space after them"
    );
}

/// A **tracked change** is not replaced.
#[test]
fn a_tracked_change_is_not_replaced() {
    let mut session = session();
    let block = a_paragraph(
        &mut session,
        "the quick brown fox",
        json!([{ "len": 4 }, { "len": 15, "revision": { "kind": "insert", "author": "a reader" } }]),
    );
    let value = session
        .execute("prose.replace", &json!({ "block": block, "what": "quick", "with": "slow" }))
        .expect_err("the replace is refused");
    assert!(value.to_string().contains("tracked change"), "the refusal says why, and it says {}", value);
    assert_eq!(text_of(&mut session, block), "the quick brown fox", "and the text is as it was");
}

/// A pattern that is **nothing** is refused.
#[test]
fn a_pattern_that_is_nothing_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session, "the quick brown fox", json!([{ "len": 19 }]));
    let value = session
        .execute("prose.find", &json!({ "block": block, "what": "" }))
        .expect_err("the find is refused");
    assert!(value.to_string().contains("nothing to find"), "the refusal says why, and it says {}", value);
}

/// A **replace** that finds nothing changes nothing.
#[test]
fn a_replace_that_finds_nothing_changes_nothing() {
    let mut session = session();
    let block = a_paragraph(&mut session, "the quick brown fox", json!([{ "len": 19 }]));
    let outcome = session
        .execute("prose.replace", &json!({ "block": block, "what": "cat", "with": "dog" }))
        .expect("the replace is refused");
    assert_eq!(outcome.data["count"], json!(0), "nothing was replaced");
    assert_eq!(text_of(&mut session, block), "the quick brown fox", "and the text is as it was");
}
