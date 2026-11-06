//! The **colour** of a PDF file.

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

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-colour-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// An **object** with a dictionary.
fn add_object(session: &mut Session, object: u64, entries: Json) {
    session
        .execute("pdf.object.add", &json!({ "object": object, "entries": entries }))
        .expect("the object is added");
}

/// A **content stream** that paints with the given instructions.
fn a_content_stream(session: &mut Session, name: &str, instructions: &str) {
    let directory = directory(name);
    let path = format!("{directory}/content.txt");
    std::fs::write(&path, instructions).expect("the bytes are written");
    session
        .execute("pdf.stream.add", &json!({ "object": 2, "entries": [], "stream": path }))
        .expect("the stream is added");
}

/// The **catalog** of a document: the object that says it is one.
fn a_catalog(session: &mut Session) {
    add_object(session, 1, json!([["Type", "Catalog"]]));
}

/// A **PDF/A output intent**: an intent that names a profile.
fn a_pdfa_intent(session: &mut Session) {
    add_object(
        session,
        3,
        json!([
            ["Type", "OutputIntent"],
            ["Subtype", "GTS_PDFA1"],
            ["OutputConditionIdentifier", "sRGB"],
            ["DestOutputProfile", { "ref": [4, 0] }],
        ]),
    );
    let catalog = session
        .document()
        .walk()
        .find(|node| node.kind == "pdf.object")
        .map(|node| node.id)
        .expect("the catalog is there");
    session
        .execute(
            "pdf.entry.set",
            &json!({ "object": catalog, "key": "OutputIntents", "value": [{ "ref": [3, 0] }] }),
        )
        .expect("the intents are named");
}

/// The **preflight** of the document.
fn preflight(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.preflight", &json!({})).expect("the preflight is done");
    outcome.data.clone()
}

/// The **clauses** the document breaks.
fn clauses(preflight: &Json) -> Vec<String> {
    preflight["issues"]
        .as_array()
        .expect("the issues are a list")
        .iter()
        .map(|issue| issue["clause"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// Device colour without an output intent breaks the rule.
#[test]
fn device_colour_without_an_output_intent_breaks_the_rule() {
    let mut session = session();
    a_catalog(&mut session);
    a_content_stream(&mut session, "rgb", "1 0 0 rg\n");
    let preflight = preflight(&mut session);
    assert!(clauses(&preflight).contains(&"6.2.3".to_string()), "the rule is broken: {preflight}");
}

/// Device colour with an output intent breaks nothing.
#[test]
fn device_colour_with_an_output_intent_breaks_nothing() {
    let mut session = session();
    a_catalog(&mut session);
    a_content_stream(&mut session, "intent", "1 0 0 rg\n");
    a_pdfa_intent(&mut session);
    let preflight = preflight(&mut session);
    assert!(!clauses(&preflight).contains(&"6.2.3".to_string()), "the intent is there: {preflight}");
}

/// **CMYK** needs an intent of its own, and it is not added.
#[test]
fn cmyk_needs_an_intent_of_its_own_and_it_is_not_added() {
    let mut session = session();
    a_catalog(&mut session);
    a_content_stream(&mut session, "cmyk", "0 0 0 1 k\n");
    let preflight = preflight(&mut session);
    let clauses = clauses(&preflight);
    assert!(clauses.contains(&"6.2.3".to_string()), "the rule is broken: {preflight}");
    assert!(clauses.contains(&"6.2.4.3".to_string()), "and the CMYK rule: {preflight}");
}

/// The **CMYK** rule cannot be fixed.
#[test]
fn the_cmyk_rule_cannot_be_fixed() {
    let mut session = session();
    a_catalog(&mut session);
    a_content_stream(&mut session, "cmykfix", "0 0 0 1 k\n");
    let preflight = preflight(&mut session);
    let issues = preflight["issues"].as_array().expect("the issues are a list");
    let issue =
        issues.iter().find(|issue| issue["clause"] == json!("6.2.4.3")).expect("the CMYK rule is broken");
    assert_eq!(issue["fixable"], json!(false), "the tree cannot fix it: {issue}");
}

/// A dictionary that names a **device colour space** says so.
#[test]
fn a_dictionary_that_names_a_device_colour_space_says_so() {
    let mut session = session();
    a_catalog(&mut session);
    add_object(
        &mut session,
        2,
        json!([["Type", "XObject"], ["Subtype", "Image"], ["ColorSpace", "DeviceCMYK"]]),
    );
    let preflight = preflight(&mut session);
    assert!(clauses(&preflight).contains(&"6.2.4.3".to_string()), "the rule is broken: {preflight}");
}

/// The **colour** the file paints in is reported.
#[test]
fn the_colour_the_file_paints_in_is_reported() {
    let mut session = session();
    a_catalog(&mut session);
    a_content_stream(&mut session, "reports", "1 0 0 rg\n");
    let preflight = preflight(&mut session);
    assert_eq!(preflight["colour"]["rgb"], json!(true), "the file paints in red, green and blue");
    assert_eq!(preflight["colour"]["cmyk"], json!(false), "and not in CMYK");
}

/// **Gray** is a device colour too, and it needs the intent as well.
#[test]
fn gray_is_a_device_colour_too_and_it_needs_the_intent_as_well() {
    let mut session = session();
    a_catalog(&mut session);
    a_content_stream(&mut session, "gray", "0.5 g\n");
    let preflight = preflight(&mut session);
    assert!(clauses(&preflight).contains(&"6.2.3".to_string()), "the rule is broken: {preflight}");
    assert_eq!(preflight["colour"]["rgb"], json!(true), "the file paints in gray");
}
