//! The **labels** of the pages.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the PDF battery.
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

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = craftkit_scratch::dir(&format!("craftkit-v2-labels-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **pages** of a document: the object that holds them.
fn the_pages(session: &mut Session) -> u64 {
    let outcome = session
        .execute("pdf.object.add", &json!({ "object": 1, "entries": [["Type", "Catalog"]] }))
        .expect("the catalog is added");
    let _ = outcome;
    let outcome = session
        .execute("pdf.object.add", &json!({ "object": 2, "entries": [["Type", "Pages"]] }))
        .expect("the pages are added");
    outcome.data["id"].as_u64().expect("the pages have an id")
}

/// The **labels** of the pages, as the command reports them.
fn labels(session: &mut Session, pages: u64) -> u64 {
    let outcome = session
        .execute(
            "pdf.page.labels",
            &json!({
                "pages": pages,
                "labels": [
                    { "page": 1, "style": "r" },
                    { "page": 3, "style": "D", "prefix": "A-", "start": 1 },
                ],
            }),
        )
        .expect("the pages are given their labels");
    outcome.data["tree"].as_u64().expect("the tree has an id")
}

/// The pages may be given their **labels**, and the file says so.
#[test]
fn the_pages_may_be_given_their_labels() {
    let mut session = session();
    let pages = the_pages(&mut session);
    let tree = labels(&mut session, pages);
    let number = session
        .document()
        .node(tree)
        .and_then(|node| node.data.get("object").and_then(|number| number.as_u64()))
        .expect("the tree has a number");
    let directory = directory("written");
    let path = format!("{directory}/the.pdf");
    session.execute("pdf.file.write", &json!({ "path": path })).expect("the file is written");
    let bytes = std::fs::read(&path).expect("the file is there");
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains(&format!("/PageLabels {number} 0 R")),
        "the pages name the tree, and the file says {}",
        text
    );
    assert!(text.contains("/Nums [0 << /S /r /St 1 >>"), "and the tree holds the runs, and it says {}", text);
}

/// The **prefix** of a label is written as text.
#[test]
fn the_prefix_of_a_label_is_written_as_text() {
    let mut session = session();
    let pages = the_pages(&mut session);
    labels(&mut session, pages);
    let directory = directory("prefix");
    let path = format!("{directory}/the.pdf");
    session.execute("pdf.file.write", &json!({ "path": path })).expect("the file is written");
    let bytes = std::fs::read(&path).expect("the file is there");
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("/P (A-)"), "the prefix is text, and the file says {}", text);
}

/// A style that is **not a style** is refused.
#[test]
fn a_style_that_is_not_a_style_is_refused() {
    let mut session = session();
    let pages = the_pages(&mut session);
    let value = session
        .execute("pdf.page.labels", &json!({ "pages": pages, "labels": [{ "page": 1, "style": "Z" }] }))
        .expect_err("the labels are refused");
    assert!(value.to_string().contains("not a style"), "the refusal says why, and it says {}", value);
}

/// The labels must be **in order**.
#[test]
fn the_labels_must_be_in_order() {
    let mut session = session();
    let pages = the_pages(&mut session);
    let value = session
        .execute(
            "pdf.page.labels",
            &json!({
                "pages": pages,
                "labels": [{ "page": 5 }, { "page": 2 }],
            }),
        )
        .expect_err("the labels are refused");
    assert!(value.to_string().contains("in order"), "the refusal says why, and it says {}", value);
}

/// A label that does not say which **page** it begins at is refused.
#[test]
fn a_label_that_does_not_say_which_page_is_refused() {
    let mut session = session();
    let pages = the_pages(&mut session);
    let value = session
        .execute("pdf.page.labels", &json!({ "pages": pages, "labels": [{ "style": "D" }] }))
        .expect_err("the labels are refused");
    assert!(value.to_string().contains("which page"), "the refusal says why, and it says {}", value);
}

/// A node that is **not the pages** is refused.
#[test]
fn a_node_that_is_not_the_pages_is_refused() {
    let mut session = session();
    let outcome = session
        .execute("pdf.object.add", &json!({ "object": 1, "entries": [["Type", "Catalog"]] }))
        .expect("the catalog is added");
    let catalog = outcome.data["id"].as_u64().expect("the catalog has an id");
    let value = session
        .execute("pdf.page.labels", &json!({ "pages": catalog, "labels": [{ "page": 1 }] }))
        .expect_err("the labels are refused");
    assert!(value.to_string().contains("not the pages"), "the refusal says why, and it says {}", value);
}
