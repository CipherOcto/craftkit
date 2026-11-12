//! The **output intent** of a file.

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

/// An **object** with a dictionary.
fn add_object(session: &mut Session, object: u64, entries: Json) {
    session
        .execute("pdf.object.add", &json!({ "object": object, "entries": entries }))
        .expect("the object is added");
}

/// The **catalog** of a document: the object that says it is one.
fn a_catalog(session: &mut Session) {
    add_object(session, 1, json!([["Type", "Catalog"]]));
}

/// A **content stream** that paints with device colour.
fn a_content_stream(session: &mut Session, name: &str) {
    let directory = directory(name);
    let path = format!("{directory}/content.txt");
    std::fs::write(&path, "1 0 0 rg\n").expect("the bytes are written");
    session
        .execute("pdf.stream.add", &json!({ "object": 2, "entries": [], "stream": path }))
        .expect("the stream is added");
}

/// A **directory** of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-intent-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
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

/// The **profile** of the intent, as the command writes it.
fn write_the_profile(session: &mut Session, name: &str) -> (String, Json) {
    let directory = directory(name);
    let path = format!("{directory}/srgb.icc");
    let outcome =
        session.execute("pdf.intent.write", &json!({ "path": path })).expect("the intent is written");
    (path, outcome.data.clone())
}

/// The profile is **well formed**.
#[test]
fn the_profile_is_well_formed() {
    let mut session = session();
    a_catalog(&mut session);
    let (path, _) = write_the_profile(&mut session, "profile");
    let bytes = std::fs::read(&path).expect("the profile is there");
    let size = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    assert_eq!(size, bytes.len(), "the header says how long the profile is");
    assert_eq!(&bytes[36..40], b"acsp", "the profile says what it is");
    assert_eq!(&bytes[12..20], b"mntrRGB ", "and what it is for");
    let count = u32::from_be_bytes([bytes[128], bytes[129], bytes[130], bytes[131]]) as usize;
    assert_eq!(count, 9, "the profile has nine tags");
    for tag in 0..count {
        let at = 132 + tag * 12;
        let offset =
            u32::from_be_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]]) as usize;
        let len = u32::from_be_bytes([bytes[at + 8], bytes[at + 9], bytes[at + 10], bytes[at + 11]]) as usize;
        assert!(offset.is_multiple_of(4), "the tag {tag} begins on a four-byte boundary");
        assert!(offset + len <= bytes.len(), "the tag {tag} is in the profile");
    }
}

/// The catalog **names the intent**.
#[test]
fn the_catalog_names_the_intent() {
    let mut session = session();
    a_catalog(&mut session);
    write_the_profile(&mut session, "catalog");
    let catalog =
        session.document().walk().find(|node| node.kind == "pdf.object").expect("the catalog is there");
    let entries = catalog.data["entries"].clone();
    let intents = entries
        .as_array()
        .expect("the entries are a list")
        .iter()
        .find_map(|entry| {
            let pair = entry.as_array()?;
            if pair.first()?.as_str()? != "OutputIntents" {
                return None;
            }
            Some(pair.get(1)?.clone())
        })
        .expect("the catalog has the intents");
    let list = intents.as_array().expect("the intents are a list");
    assert_eq!(list.len(), 1, "there is one intent");
}

/// The intent says what it is with the **name the file uses**.
#[test]
fn the_intent_says_what_it_is_with_the_name_the_file_uses() {
    let mut session = session();
    a_catalog(&mut session);
    let (_, data) = write_the_profile(&mut session, "subtype");
    let number = data["object"].as_u64().expect("the intent has a number");
    let intent = session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|held| held.as_u64()) == Some(number))
        .expect("the intent is there");
    let entries = intent.data["entries"].clone();
    let mut found: Vec<String> = Vec::new();
    for entry in entries.as_array().expect("the entries are a list") {
        let pair = entry.as_array().expect("an entry is a pair");
        found.push(pair.first().and_then(|key| key.as_str()).unwrap_or_default().to_string());
    }
    assert!(found.contains(&"S".to_string()), "the intent says its subtype with S: {found:?}");
    assert!(found.contains(&"Type".to_string()), "and says what it is with Type");
    assert!(found.contains(&"DestOutputProfile".to_string()), "and names the profile");
}

/// The checker stops saying the colour needs an **output intent**.
#[test]
fn the_checker_stops_saying_the_colour_needs_an_output_intent() {
    let mut session = session();
    a_catalog(&mut session);
    a_content_stream(&mut session, "colour");
    assert!(clauses_of(&mut session).contains(&"6.2.3".to_string()), "the rule is broken");
    write_the_profile(&mut session, "colour");
    let clauses = clauses_of(&mut session);
    assert!(!clauses.contains(&"6.2.3".to_string()), "the rule is not broken: {clauses:?}");
}

/// A path that is **empty** is refused.
#[test]
fn a_path_that_is_empty_is_refused() {
    let mut session = session();
    a_catalog(&mut session);
    let refused =
        session.execute("pdf.intent.write", &json!({ "path": "  " })).expect_err("the path is refused");
    assert!(refused.to_string().contains("bytes go somewhere"), "the refusal says why: {refused}");
}

/// A file with **no catalog** is refused.
#[test]
fn a_file_with_no_catalog_is_refused() {
    let mut session = session();
    let directory = directory("no-catalog");
    let refused = session
        .execute("pdf.intent.write", &json!({ "path": format!("{directory}/srgb.icc") }))
        .expect_err("the file is refused");
    assert!(refused.to_string().contains("no catalog"), "the refusal says why: {refused}");
}
