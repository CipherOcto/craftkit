//! The **information** of a document.

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
    let path =
        craftkit_scratch::dir(&format!("craftkit-v2-information-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **catalog** and an **object** to be the information.
fn a_document(session: &mut Session) -> (u64, u64) {
    let outcome = session
        .execute("pdf.object.add", &json!({ "object": 1, "entries": [["Type", "Catalog"]] }))
        .expect("the catalog is added");
    let catalog = outcome.data["id"].as_u64().expect("the catalog has an id");
    let outcome = session
        .execute(
            "pdf.object.add",
            &json!({ "object": 2, "entries": [["Title", "the title"], ["Author", "the author"]] }),
        )
        .expect("the information is added");
    let information = outcome.data["id"].as_u64().expect("the object has an id");
    (catalog, information)
}

/// An object may be made the file's **information**.
#[test]
fn an_object_may_be_made_the_information() {
    let mut session = session();
    let (_catalog, information) = a_document(&mut session);
    let outcome = session
        .execute("pdf.info.set", &json!({ "object": information }))
        .expect("the object is the information");
    assert_eq!(outcome.data["number"], json!(2), "and it is the object two");
}

/// The **trailer** of the file names the information.
#[test]
fn the_trailer_names_the_information() {
    let mut session = session();
    let (_catalog, information) = a_document(&mut session);
    session
        .execute("pdf.info.set", &json!({ "object": information }))
        .expect("the object is the information");
    let directory = directory("trailer");
    let path = format!("{directory}/the.pdf");
    session.execute("pdf.file.write", &json!({ "path": path })).expect("the file is written");
    let bytes = std::fs::read(&path).expect("the file is there");
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("/Info 2 0 R"), "the trailer names the information, and it says {}", text);
}

/// A file with **no information** says nothing about it in its
/// trailer.
#[test]
fn a_file_with_no_information_says_nothing_about_it() {
    let mut session = session();
    a_document(&mut session);
    let directory = directory("none");
    let path = format!("{directory}/the.pdf");
    session.execute("pdf.file.write", &json!({ "path": path })).expect("the file is written");
    let bytes = std::fs::read(&path).expect("the file is there");
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("/Info"), "the trailer says nothing, and it says {}", text);
}

/// There is **one** information dictionary in a file, and a second one
/// is refused.
#[test]
fn a_second_information_dictionary_is_refused() {
    let mut session = session();
    let (_catalog, information) = a_document(&mut session);
    session
        .execute("pdf.info.set", &json!({ "object": information }))
        .expect("the object is the information");
    let outcome = session
        .execute("pdf.object.add", &json!({ "object": 3, "entries": [["Title", "another"]] }))
        .expect("the object is added");
    let other = outcome.data["id"].as_u64().expect("the object has an id");
    let value = session
        .execute("pdf.info.set", &json!({ "object": other }))
        .expect_err("the second information is refused");
    assert!(
        value.to_string().contains("already has the information"),
        "the refusal says why, and it says {}",
        value
    );
}

/// A node that is **not an object** is refused.
#[test]
fn a_node_that_is_not_an_object_is_refused() {
    let mut session = session();
    let (_catalog, information) = a_document(&mut session);
    let value = session
        .execute("pdf.info.set", &json!({ "object": information + 1000 }))
        .expect_err("the information is refused");
    assert!(value.to_string().contains("there is no node"), "the refusal says why, and it says {}", value);
}
