//! The **articles** of a layout document.
//!
//! An article is a named list of items in a reading order, and the
//! rules are the tree's own.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_layout::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the layout battery.
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

/// A **document** with a **spread**, a **page** and **two** items in
/// it, and their ids.
fn a_document(session: &mut Session) -> (u64, u64, u64) {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    let mut items = Vec::new();
    for name in ["first", "second"] {
        let outcome = session
            .execute(
                "layout.item.add",
                &json!({ "spread": spread, "name": name, "kind": "layout.item.frame" }),
            )
            .expect("the item is added");
        items.push(outcome.data["id"].as_u64().expect("the item has an id"));
    }
    (document, items[0], items[1])
}

/// The **articles** of a document, as the answer holds them.
fn articles_of(session: &mut Session, document: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("layout.document.articles", &json!({ "document": document }))
        .expect("the articles are read");
    outcome.data["articles"].as_array().expect("the articles are a list").clone()
}

/// An article holds the **items** it is given, in the order they are
/// given.
#[test]
fn an_article_holds_its_items() {
    let mut session = session();
    let (document, first, second) = a_document(&mut session);
    let outcome = session
        .execute(
            "layout.article.new",
            &json!({ "document": document, "name": "Main", "items": [second, first] }),
        )
        .expect("the article is added");
    let article = outcome.data["id"].as_u64().expect("the article has an id");
    assert_eq!(outcome.data["name"], json!("Main"), "it is named: {outcome:?}");
    let articles = articles_of(&mut session, document);
    assert_eq!(articles.len(), 1, "there is one article: {articles:?}");
    assert_eq!(articles[0]["items"], json!([second, first]), "and it holds them in order: {articles:?}");
    assert_eq!(articles[0]["export"], json!(true), "and it is exported: {articles:?}");
    assert_eq!(articles[0]["id"], json!(article), "the answer names it: {outcome:?}");
}

/// An article that is added without a **name** is called `Article N`,
/// and a name that is **taken** is refused.
#[test]
fn the_name_of_an_article_is_the_tree_s_own() {
    let mut session = session();
    let (document, _first, _second) = a_document(&mut session);
    let outcome = session
        .execute("layout.article.new", &json!({ "document": document }))
        .expect("the article is added");
    assert_eq!(outcome.data["name"], json!("Article 1"), "it is the first: {outcome:?}");
    session
        .execute("layout.article.new", &json!({ "document": document, "name": "Main" }))
        .expect("the article is added");
    let outcome = session
        .execute("layout.article.new", &json!({ "document": document, "name": "Main" }))
        .expect_err("the name is taken");
    assert!(outcome.to_string().contains("an article named `Main` exists"), "it says why: {outcome}");
}

/// The items of an article are **added** and **taken out**, and an item
/// is not in an article twice.
#[test]
fn the_items_of_an_article_are_added_and_taken_out() {
    let mut session = session();
    let (document, first, second) = a_document(&mut session);
    let outcome = session
        .execute("layout.article.new", &json!({ "document": document, "name": "Main" }))
        .expect("the article is added");
    let article = outcome.data["id"].as_u64().expect("the article has an id");
    let outcome = session
        .execute("layout.article.add", &json!({ "article": article, "items": [first] }))
        .expect("the item is added");
    assert_eq!(outcome.data["added"], json!(1), "it is added: {outcome:?}");
    // The same item again is not added twice.
    let outcome = session
        .execute("layout.article.add", &json!({ "article": article, "items": [first] }))
        .expect("the item is added");
    assert_eq!(outcome.data["added"], json!(0), "and not twice: {outcome:?}");
    let articles = articles_of(&mut session, document);
    assert_eq!(articles[0]["items"], json!([first]), "the article holds it once: {articles:?}");
    // An item that is **not there** is not in the article.
    let outcome = session
        .execute("layout.article.add", &json!({ "article": article, "items": [9999] }))
        .expect("the items are read");
    assert_eq!(outcome.data["added"], json!(0), "and it is not there: {outcome:?}");
    // The item is **taken out**.
    let outcome = session
        .execute("layout.article.remove", &json!({ "article": article, "items": [first] }))
        .expect("the item is taken out");
    assert_eq!(outcome.data["removed"], json!(1), "it is taken out: {outcome:?}");
    let articles = articles_of(&mut session, document);
    assert_eq!(articles[0]["items"], json!([]), "and the article is empty: {articles:?}");
    let _ = second;
}

/// The **order** of the items may be set: the items that are named
/// come first, and the rest follow.
#[test]
fn the_order_of_the_items_may_be_set() {
    let mut session = session();
    let (document, first, second) = a_document(&mut session);
    let outcome = session
        .execute(
            "layout.article.new",
            &json!({ "document": document, "name": "Main", "items": [first, second] }),
        )
        .expect("the article is added");
    let article = outcome.data["id"].as_u64().expect("the article has an id");
    session
        .execute("layout.article.options", &json!({ "article": article, "order": [second] }))
        .expect("the order is set");
    let articles = articles_of(&mut session, document);
    assert_eq!(articles[0]["items"], json!([second, first]), "the named item comes first: {articles:?}");
}

/// An article may be **renamed**, it may say whether it is
/// **exported**, and it may be **deleted**.
#[test]
fn an_article_may_be_renamed_and_deleted() {
    let mut session = session();
    let (document, _first, _second) = a_document(&mut session);
    let outcome = session
        .execute("layout.article.new", &json!({ "document": document, "name": "Main" }))
        .expect("the article is added");
    let article = outcome.data["id"].as_u64().expect("the article has an id");
    session
        .execute("layout.article.options", &json!({ "article": article, "to": "Front", "export": false }))
        .expect("the options are set");
    let articles = articles_of(&mut session, document);
    assert_eq!(articles[0]["name"], json!("Front"), "it is renamed: {articles:?}");
    assert_eq!(articles[0]["export"], json!(false), "and it is not exported: {articles:?}");
    session.execute("layout.article.delete", &json!({ "article": article })).expect("the article is deleted");
    assert!(articles_of(&mut session, document).is_empty(), "and it is gone");
    let error = session
        .execute("layout.article.delete", &json!({ "article": article }))
        .expect_err("the article is not there");
    assert!(error.to_string().contains("there is no article"), "it says why: {error}");
}
