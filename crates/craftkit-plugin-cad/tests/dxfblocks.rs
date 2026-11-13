//! The **blocks** of a DXF file: the blocks of a file and the inserts
//! that put them in a drawing.
//!
//! The file is a DXF with a `BLOCKS` section, and the drawing it
//! names has the blocks of the file in it.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_cad::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the cad battery.
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

/// A path of this test's own, so that two tests do not fight over one
/// file.
fn a_path(name: &str) -> String {
    craftkit_scratch::file(&format!("craftkit-v2-dxfblocks-{name}-{}", std::process::id()))
        .expect("the scratch is made")
        .to_string_lossy()
        .to_string()
}

/// **Write** a file of this test's own.
fn a_file(name: &str, text: &str) -> String {
    let path = a_path(name);
    std::fs::write(&path, text.as_bytes()).expect("the file is written");
    path
}

/// The **blocks** of the drawing that was read, by name.
fn blocks_of(session: &Session, drawing: u64) -> Vec<(String, u64)> {
    let node = session.document().node(drawing).expect("the drawing is there");
    node.children
        .iter()
        .filter(|child| child.kind == "cad.block")
        .map(|child| (child.name.clone(), child.id))
        .collect()
}

/// The **entities** of a block, by kind.
fn entities_of(session: &Session, block: u64) -> Vec<(String, Json)> {
    let node = session.document().node(block).expect("the block is there");
    node.children.iter().map(|child| (child.kind.clone(), child.data["geometry"].clone())).collect()
}

/// The **blocks** of a file are blocks of the drawing, and an **insert**
/// is an instance of one of them.
#[test]
fn the_blocks_of_a_file_are_blocks_of_the_drawing() {
    let mut session = session();
    let path = a_file(
        "blocks",
        "0\nSECTION\n2\nBLOCKS\n\
         0\nBLOCK\n2\nthe chair\n10\n0.0\n20\n0.0\n\
         0\nLINE\n8\nthe walls\n10\n0.0\n20\n0.0\n11\n1.0\n21\n0.0\n\
         0\nENDBLK\n\
         0\nENDSEC\n\
         0\nSECTION\n2\nENTITIES\n\
         0\nLINE\n8\nthe walls\n10\n0.0\n20\n0.0\n11\n2.0\n21\n0.0\n\
         0\nINSERT\n8\nthe walls\n2\nthe chair\n10\n5.0\n20\n5.0\n\
         0\nENDSEC\n\
         0\nEOF\n",
    );
    let outcome = session
        .execute("cad.import.dxf", &json!({ "path": path, "name": "the drawing" }))
        .expect("the file is read");
    // The line of the model space, the line of the chair and the
    // insert of the chair.
    assert_eq!(outcome.data["entities"], json!(3), "three entities are read: {outcome:?}");
    assert_eq!(outcome.data["blocks"], json!(2), "there are two blocks: {outcome:?}");
    let drawing = outcome.data["drawing"].as_u64().expect("the drawing has an id");
    let blocks = blocks_of(&session, drawing);
    let chair = blocks.iter().find(|(name, _)| name == "the chair").expect("the chair is there");
    let entities = entities_of(&session, chair.1);
    assert_eq!(entities.len(), 1, "the chair holds one line: {entities:?}");
    assert_eq!(entities[0].0, "cad.entity.line", "and it is a line: {entities:?}");
    // The **model space** holds the line and the insert.
    let model = blocks.iter().find(|(name, _)| name == "*Model_Space").expect("the model is there");
    let entities = entities_of(&session, model.1);
    assert_eq!(entities.len(), 2, "the model holds two entities: {entities:?}");
    assert_eq!(entities[1].0, "cad.entity.insert", "the second is the insert: {entities:?}");
    assert_eq!(
        entities[1].1["at"],
        json!({ "x": 5.0, "y": 5.0 }),
        "and it is put where the file says: {entities:?}"
    );
    // The **edge** from the instance to its definition.
    let node = session.document().node(model.1).expect("the block is there");
    let insert = node.children.iter().find(|child| child.kind == "cad.entity.insert").expect("it is there");
    let relations = session.document().relations_from(insert.id, Some("cad.inserts"));
    assert_eq!(relations.len(), 1, "the insert names its block: {relations:?}");
    assert_eq!(relations[0].to, chair.1, "and it is the chair: {relations:?}");
}

/// An insert of a block the file does not define is **skipped**, and
/// the answer says what it skipped.
#[test]
fn an_insert_of_a_block_that_is_not_there_is_skipped() {
    let mut session = session();
    let path = a_file(
        "missing",
        "0\nSECTION\n2\nENTITIES\n\
         0\nINSERT\n8\nthe walls\n2\nthe table\n10\n5.0\n20\n5.0\n\
         0\nENDSEC\n\
         0\nEOF\n",
    );
    let outcome = session.execute("cad.import.dxf", &json!({ "path": path })).expect("the file is read");
    assert_eq!(outcome.data["entities"], json!(0), "nothing is read: {outcome:?}");
    let skipped = outcome.data["skipped"].as_array().expect("the skipped are a list");
    assert_eq!(skipped, &vec![json!("INSERT")], "the insert is said: {skipped:?}");
}

/// A file that keeps its entities in the **model's own record** is
/// read, when it has no entities of its own.
#[test]
fn the_entities_of_the_model_s_own_record_are_read() {
    let mut session = session();
    let path = a_file(
        "model",
        "0\nSECTION\n2\nBLOCKS\n\
         0\nBLOCK\n2\n*Model_Space\n10\n0.0\n20\n0.0\n\
         0\nLINE\n8\nthe walls\n10\n0.0\n20\n0.0\n11\n3.0\n21\n0.0\n\
         0\nENDBLK\n\
         0\nENDSEC\n\
         0\nEOF\n",
    );
    let outcome = session.execute("cad.import.dxf", &json!({ "path": path })).expect("the file is read");
    assert_eq!(outcome.data["entities"], json!(1), "the line is read: {outcome:?}");
    let drawing = outcome.data["drawing"].as_u64().expect("the drawing has an id");
    let blocks = blocks_of(&session, drawing);
    assert_eq!(blocks.len(), 1, "there is one block: {blocks:?}");
    let entities = entities_of(&session, blocks[0].1);
    assert_eq!(entities.len(), 1, "and it holds the line: {entities:?}");
    assert_eq!(
        entities[0].1["to"],
        json!({ "x": 3.0, "y": 0.0 }),
        "with the end the file says: {entities:?}"
    );
}
