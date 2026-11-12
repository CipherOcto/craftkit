//! The **preflight** of a PDF file.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-preflight-{name}-{}", std::process::id()));
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

/// The **catalog** of a document: the object that says it is one.
fn a_catalog(session: &mut Session) {
    add_object(session, 1, json!([["Type", "Catalog"]]));
}

/// The **metadata** of a document: the XMP the catalog names.
fn a_metadata(session: &mut Session, name: &str, xmp: &str) {
    let directory = directory(name);
    let path = format!("{directory}/metadata.xmp");
    std::fs::write(&path, xmp).expect("the bytes are written");
    session
        .execute(
            "pdf.stream.add",
            &json!({ "object": 2, "entries": [["Type", "Metadata"], ["Subtype", "XML"]], "stream": path }),
        )
        .expect("the stream is added");
    add_object(session, 1, json!([["Type", "Catalog"], ["Metadata", { "ref": [2, 0] }]]));
}

/// The **preflight** of the document.
fn preflight(session: &mut Session, level: Option<&str>) -> Json {
    let mut params = json!({});
    if let Some(level) = level {
        params["level"] = json!(level);
    }
    let outcome = session.execute("pdf.preflight", &params).expect("the preflight is done");
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

/// A document with **no metadata** breaks the metadata rule.
#[test]
fn a_document_with_no_metadata_breaks_the_metadata_rule() {
    let mut session = session();
    a_catalog(&mut session);
    let preflight = preflight(&mut session, None);
    assert!(clauses(&preflight).contains(&"6.6.2.1".to_string()), "the rule is broken: {preflight}");
}

/// A document with metadata says **what it declares**.
#[test]
fn a_document_with_metadata_says_what_it_declares() {
    let mut session = session();
    a_metadata(
        &mut session,
        "declares",
        r#"<?xpacket?><x:xmpmeta><rdf:RDF><rdf:Description pdfaid:part="2" pdfaid:conformance="B"/></rdf:RDF></x:xmpmeta>"#,
    );
    let preflight = preflight(&mut session, None);
    assert_eq!(preflight["pdfa"]["part"], json!(2), "the part is declared");
    assert_eq!(preflight["pdfa"]["conformance"], json!("B"), "and the conformance");
    assert!(!clauses(&preflight).contains(&"6.6.2.1".to_string()), "the metadata is there");
}

/// A document that declares the **right level** breaks nothing.
#[test]
fn a_document_that_declares_the_right_level_breaks_nothing() {
    let mut session = session();
    a_metadata(
        &mut session,
        "right",
        r#"<x:xmpmeta><rdf:RDF><rdf:Description pdfaid:part="3" pdfaid:conformance="B"/></rdf:RDF></x:xmpmeta>"#,
    );
    let preflight = preflight(&mut session, Some("3b"));
    assert!(!clauses(&preflight).contains(&"6.6.4".to_string()), "the file says what it is: {preflight}");
}

/// A document that declares the **wrong level** breaks the rule.
#[test]
fn a_document_that_declares_the_wrong_level_breaks_the_rule() {
    let mut session = session();
    a_metadata(
        &mut session,
        "wrong",
        r#"<x:xmpmeta><rdf:RDF><rdf:Description pdfaid:part="2" pdfaid:conformance="B"/></rdf:RDF></x:xmpmeta>"#,
    );
    let preflight = preflight(&mut session, Some("3b"));
    assert!(clauses(&preflight).contains(&"6.6.4".to_string()), "the rule is broken: {preflight}");
}

/// A **JavaScript** action is not allowed.
#[test]
fn a_javascript_action_is_not_allowed() {
    let mut session = session();
    a_catalog(&mut session);
    add_object(&mut session, 2, json!([["Type", "Action"], ["S", "JavaScript"]]));
    let preflight = preflight(&mut session, None);
    assert!(clauses(&preflight).contains(&"6.5.1".to_string()), "the rule is broken: {preflight}");
}

/// A **Sound** annotation is not allowed.
#[test]
fn a_sound_annotation_is_not_allowed() {
    let mut session = session();
    a_catalog(&mut session);
    add_object(
        &mut session,
        2,
        json!([
            ["Type", "Annot"],
            ["Subtype", "Sound"],
            ["Rect", [0.0, 0.0, 10.0, 10.0]],
            ["P", { "ref": [1, 0] }],
        ]),
    );
    let preflight = preflight(&mut session, None);
    assert!(clauses(&preflight).contains(&"6.3.1".to_string()), "the rule is broken: {preflight}");
}

/// **Additional actions** are not allowed.
#[test]
fn additional_actions_are_not_allowed() {
    let mut session = session();
    a_catalog(&mut session);
    add_object(&mut session, 2, json!([["Type", "Action"], ["S", "Named"], ["AA", { "ref": [3, 0] }]]));
    let preflight = preflight(&mut session, None);
    assert!(clauses(&preflight).contains(&"6.5.2".to_string()), "the rule is broken: {preflight}");
}

/// The **output intents** are read.
#[test]
fn the_output_intents_are_read() {
    let mut session = session();
    a_catalog(&mut session);
    add_object(&mut session, 2, json!([["Type", "OutputIntent"], ["OutputConditionIdentifier", "sRGB"]]));
    let catalog = session
        .document()
        .walk()
        .find(|node| node.kind == "pdf.object")
        .map(|node| node.id)
        .expect("the catalog is there");
    session
        .execute(
            "pdf.entry.set",
            &json!({ "object": catalog, "key": "OutputIntents", "value": [{ "ref": [2, 0] }] }),
        )
        .expect("the intents are named");
    let preflight = preflight(&mut session, None);
    assert_eq!(preflight["output_intents"], json!(["sRGB"]), "the intent is named");
}

/// A level that is **not a level** is refused.
#[test]
fn a_level_that_is_not_a_level_is_refused() {
    let mut session = session();
    a_catalog(&mut session);
    let refused =
        session.execute("pdf.preflight", &json!({ "level": "4z" })).expect_err("the level is refused");
    assert!(refused.to_string().contains("not a level"), "the refusal says why: {refused}");
}
