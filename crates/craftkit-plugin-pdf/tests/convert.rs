//! The **convert** of a PDF file.

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

/// **Set** a key of an object's dictionary.
fn set_entry(session: &mut Session, object: u64, key: &str, value: Json) {
    let node = session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|number| number.as_u64()) == Some(object))
        .map(|node| node.id)
        .expect("the object is there");
    session
        .execute("pdf.entry.set", &json!({ "object": node, "key": key, "value": value }))
        .expect("the entry is set");
}

/// The **entries** of an object, as the document holds them.
fn entries_of(session: &Session, object: u64) -> Vec<(String, Json)> {
    session
        .document()
        .walk()
        .find(|node| node.data.get("object").and_then(|number| number.as_u64()) == Some(object))
        .map(|node| {
            node.data
                .get("entries")
                .and_then(|entries| entries.as_array())
                .map(|list| {
                    list.iter()
                        .filter_map(|pair| {
                            let pair = pair.as_array()?;
                            Some((pair.first()?.as_str()?.to_string(), pair.get(1)?.clone()))
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
        .unwrap_or_default()
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

/// The **convert** of the document.
fn convert(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.convert", &json!({})).expect("the file is converted");
    outcome.data.clone()
}

/// The **additional actions** are taken away.
#[test]
fn the_additional_actions_are_taken_away() {
    let mut session = session();
    add_object(&mut session, 1, json!([["Type", "Catalog"]]));
    add_object(&mut session, 2, json!([["Type", "Action"], ["S", "Named"]]));
    set_entry(&mut session, 1, "AA", json!({ "ref": [2, 0] }));
    let before = preflight(&mut session);
    assert!(clauses(&before).contains(&"6.5.2".to_string()), "the rule is broken: {before}");
    convert(&mut session);
    let after = preflight(&mut session);
    assert!(!clauses(&after).contains(&"6.5.2".to_string()), "the actions are gone: {after}");
    let entries = entries_of(&session, 1);
    assert!(!entries.iter().any(|(key, _)| key == "AA"), "and the key is gone: {entries:?}");
}

/// The **forbidden actions** are taken away.
#[test]
fn the_forbidden_actions_are_taken_away() {
    let mut session = session();
    add_object(&mut session, 1, json!([["Type", "Catalog"]]));
    add_object(&mut session, 2, json!([["Type", "Action"], ["S", "JavaScript"]]));
    let before = preflight(&mut session);
    assert!(clauses(&before).contains(&"6.5.1".to_string()), "the rule is broken: {before}");
    convert(&mut session);
    let after = preflight(&mut session);
    assert!(!clauses(&after).contains(&"6.5.1".to_string()), "the action is gone: {after}");
    assert!(entries_of(&session, 2).is_empty(), "and the object is gone");
}

/// The entry that pointed at a **forbidden action** is taken away.
#[test]
fn the_entry_that_pointed_at_a_forbidden_action_is_taken_away() {
    let mut session = session();
    add_object(&mut session, 1, json!([["Type", "Catalog"]]));
    add_object(&mut session, 2, json!([["Type", "Action"], ["S", "JavaScript"]]));
    set_entry(&mut session, 1, "A", json!({ "ref": [2, 0] }));
    convert(&mut session);
    let entries = entries_of(&session, 1);
    assert!(!entries.iter().any(|(key, _)| key == "A"), "the entry is gone: {entries:?}");
}

/// The **NeedAppearances** flag is taken away.
#[test]
fn the_need_appearances_flag_is_taken_away() {
    let mut session = session();
    add_object(&mut session, 1, json!([["Type", "Catalog"]]));
    add_object(&mut session, 3, json!([["NeedAppearances", true]]));
    set_entry(&mut session, 1, "AcroForm", json!({ "ref": [3, 0] }));
    let before = preflight(&mut session);
    assert!(clauses(&before).contains(&"6.4.1".to_string()), "the rule is broken: {before}");
    convert(&mut session);
    let after = preflight(&mut session);
    assert!(!clauses(&after).contains(&"6.4.1".to_string()), "the flag is gone: {after}");
}

/// The **report** says what was fixed.
#[test]
fn the_report_says_what_was_fixed() {
    let mut session = session();
    add_object(&mut session, 1, json!([["Type", "Catalog"]]));
    add_object(&mut session, 2, json!([["Type", "Action"], ["S", "JavaScript"]]));
    let report = convert(&mut session);
    let fixed = report["fixed"].as_array().expect("the fixes are a list");
    assert!(!fixed.is_empty(), "something was fixed: {report}");
}

/// The **report** says what is left.
#[test]
fn the_report_says_what_is_left() {
    let mut session = session();
    add_object(&mut session, 1, json!([["Type", "Catalog"]]));
    let report = convert(&mut session);
    let remaining = report["remaining"].as_array().expect("the rest is a list");
    assert!(
        remaining.iter().any(|issue| issue["clause"] == json!("6.6.2.1")),
        "the metadata is still missing: {report}"
    );
}

/// The **encryption** is not taken away.
#[test]
fn the_encryption_is_not_taken_away() {
    let mut session = session();
    let directory = craftkit_scratch::dir(&format!("craftkit-v2-convert-{}", std::process::id()))
        .expect("the directory is made");
    let path = directory.join("small.pdf");
    std::fs::write(&path, an_encrypted_file()).expect("the file is written");
    let into = directory.join("streams");
    session
        .execute("pdf.file.load", &json!({ "path": path.to_string_lossy(), "into": into.to_string_lossy() }))
        .expect("the file is loaded");
    let report = convert(&mut session);
    let remaining = report["remaining"].as_array().expect("the rest is a list");
    assert!(
        remaining.iter().any(|issue| issue["clause"] == json!("6.1.3")),
        "the file is still encrypted: {report}"
    );
}

/// A small **file** whose trailer says it is encrypted.
fn an_encrypted_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>"),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 4\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R /Encrypt 4 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// A level that is **not a level** is refused.
#[test]
fn a_level_that_is_not_a_level_is_refused() {
    let mut session = session();
    add_object(&mut session, 1, json!([["Type", "Catalog"]]));
    let refused =
        session.execute("pdf.convert", &json!({ "level": "4z" })).expect_err("the level is refused");
    assert!(refused.to_string().contains("not a level"), "the refusal says why: {refused}");
}
