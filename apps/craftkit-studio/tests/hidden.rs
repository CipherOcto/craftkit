//! A node may be **hidden**: in the document, and not on the screen.
//!
//! The flag is the **ninth** version's field, and the chain of
//! versions is the thing this file is about: a file of an earlier
//! version says nothing about hiding, and a file of the ninth version
//! says exactly what it means.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Document, Ecosystem};

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A document with one node, hidden.
fn hidden() -> Document {
    let mut document = Document::new("hidden");
    let id = document.add_node("rect", "geometry.rect", None);
    document.node_mut(id).expect("the node is there").hidden = true;
    document
}

/// A document with one node, shown.
fn shown() -> Document {
    let mut document = Document::new("shown");
    document.add_node("rect", "geometry.rect", None);
    document
}

/// The **newest** version of the format.
fn newest(ecosystem: &Ecosystem) -> craftkit_sdk::format::FormatSpec {
    let formats = &ecosystem.formats;
    formats
        .all()
        .iter()
        .filter(|format| format.tag.starts_with("craftkit/"))
        .max_by_key(|format| format.version)
        .copied()
        .expect("there is a format")
}

/// The version with the number given.
fn version(ecosystem: &Ecosystem, number: u32) -> craftkit_sdk::format::FormatSpec {
    formats_of(ecosystem).into_iter().find(|format| format.version == number).expect("the version is there")
}

/// Every format of the kit's own family.
fn formats_of(ecosystem: &Ecosystem) -> Vec<craftkit_sdk::format::FormatSpec> {
    ecosystem.formats.all().iter().filter(|format| format.tag.starts_with("craftkit/")).cloned().collect()
}

/// A hidden node **survives the file**.
///
/// The flag is written by the ninth version and read back by the ninth
/// version, and a node that was hidden is hidden.
#[test]
fn a_hidden_node_survives_the_file() {
    let ecosystem = ecosystem();
    let document = hidden();
    let written = craftkit_sdk::format::write_document(&document, &newest(&ecosystem));
    assert_eq!(written["nodes"][0]["hidden"], json!(true), "the flag is in the file");
    let read = craftkit_sdk::format::read_document(&written, &ecosystem.formats, &ecosystem.migrations)
        .expect("the document is read back");
    assert_eq!(read, document, "the document came back whole");
}

/// A file of the **eighth** version does not carry the flag.
///
/// A file of a version must not have a field that version cannot
/// explain.
#[test]
fn a_file_of_the_eighth_version_does_not_carry_the_flag() {
    let ecosystem = ecosystem();
    let written = craftkit_sdk::format::write_document(&hidden(), &version(&ecosystem, 8));
    assert!(written["nodes"][0].get("hidden").is_none(), "the eighth version does not know the flag");
}

/// A file of the **eighth** version reads as a document whose nodes are
/// all shown.
///
/// The reader reads a file that says nothing about the flag as a node
/// that is not hidden: there is no other word for it.
#[test]
fn an_old_file_reads_as_shown() {
    let ecosystem = ecosystem();
    let written = craftkit_sdk::format::write_document(&shown(), &version(&ecosystem, 8));
    let read = craftkit_sdk::format::read_document(&written, &ecosystem.formats, &ecosystem.migrations)
        .expect("the document is read back");
    assert!(!read.nodes[0].hidden, "the node is shown");
}

/// The **version** is derived from the document.
///
/// A document with a hidden node needs the ninth version, because the
/// eighth version cannot say it: the version is a fact about the
/// document and not about the program.
#[test]
fn the_version_is_derived_from_the_document() {
    let ecosystem = ecosystem();
    let (written, version) = craftkit_sdk::format::write_document_oldest(
        &hidden(),
        &ecosystem.formats,
        &ecosystem.migrations,
        "craftkit",
    )
    .expect("the document is written");
    assert_eq!(version, 9, "a hidden node needs the ninth version");
    assert_eq!(written["nodes"][0]["hidden"], json!(true), "and the flag is in the file");
}

/// The **commands** hide and show.
///
/// They are two commands and not one with a boolean: a boolean says
/// "yes" or "no" instead of saying what to do, and a command that
/// hides and shows in one needs the caller to know that.
#[test]
fn the_commands_hide_and_show() {
    let mut session = ecosystem().session();
    session
        .execute("node.add", &json!({ "name": "a shape", "kind": "geometry.rect" }))
        .expect("the node is added");
    assert!(!session.document().node(1).expect("the node is there").hidden, "the node is shown");

    session.execute("node.hide", &json!({ "node": 1 })).expect("the node is hidden");
    assert!(session.document().node(1).expect("the node is there").hidden, "the node is hidden");

    session.execute("node.show", &json!({ "node": 1 })).expect("the node is shown");
    assert!(!session.document().node(1).expect("the node is there").hidden, "the node is shown again");
}

/// Hiding a **group** hides what is in it.
///
/// The group is the thing that is hidden, and the walk does not go
/// into it.
#[test]
fn hiding_a_group_hides_what_is_in_it() {
    let mut session = ecosystem().session();
    session
        .execute("node.add", &json!({ "name": "a group", "kind": "core.group" }))
        .expect("the group is added");
    session
        .execute("node.add", &json!({ "name": "a shape", "kind": "geometry.rect", "parent": 1 }))
        .expect("the shape is added under the group");

    assert_eq!(session.document().walk_visible().count(), 2, "both are shown");
    session.execute("node.hide", &json!({ "node": 1 })).expect("the group is hidden");
    assert_eq!(session.document().walk_visible().count(), 0, "what is in it is hidden too");
}
