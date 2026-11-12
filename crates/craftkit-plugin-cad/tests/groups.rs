//! The **groups** of a drawing.

use craftkit_sdk::serde_json::json;
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

/// A **drawing** with a **block** in it: the drawing and the block.
fn a_block(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("cad.drawing.add", &json!({ "name": "the drawing" })).expect("the drawing is added");
    let drawing = outcome.data["id"].as_u64().expect("the drawing has an id");
    let outcome = session
        .execute("cad.block.add", &json!({ "drawing": drawing, "name": "the model" }))
        .expect("the block is added");
    let block = outcome.data["id"].as_u64().expect("the block has an id");
    (drawing, block)
}

/// A **line** in a block, from one point to another.
fn a_line(session: &mut Session, block: u64, name: &str, from: (f64, f64), to: (f64, f64)) -> u64 {
    let outcome = session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": block,
                "name": name,
                "from": { "x": from.0, "y": from.1 },
                "to": { "x": to.0, "y": to.1 },
            }),
        )
        .expect("the line is added");
    outcome.data["id"].as_u64().expect("the line has an id")
}

/// A group may be made of entities.
#[test]
fn a_group_may_be_made_of_entities() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let first = a_line(&mut session, block, "the first", (0.0, 0.0), (10.0, 0.0));
    let second = a_line(&mut session, block, "the second", (0.0, 5.0), (10.0, 5.0));
    let outcome = session
        .execute("cad.group.add", &json!({ "name": "the pair", "block": block, "members": [first, second] }))
        .expect("the group is made");
    assert_eq!(outcome.data["name"], json!("the pair"), "the group has the name it was given");
    assert_eq!(outcome.data["count"], json!(2), "and it holds two members");
    let members = outcome.data["members"].as_array().expect("the members are a list");
    assert_eq!(members[0], json!(first), "the first member is there");
    assert_eq!(members[1], json!(second), "and the second one");
}

/// The groups of a drawing are read.
#[test]
fn the_groups_of_a_drawing_are_read() {
    let mut session = session();
    let (drawing, block) = a_block(&mut session);
    let line = a_line(&mut session, block, "the line", (0.0, 0.0), (10.0, 0.0));
    session
        .execute("cad.group.add", &json!({ "name": "the group", "block": block, "members": [line] }))
        .expect("the group is made");
    let outcome =
        session.execute("cad.group.list", &json!({ "drawing": drawing })).expect("the groups are read");
    assert_eq!(outcome.data["count"], json!(1), "the drawing holds one group");
    let listed = outcome.data["groups"].as_array().expect("the groups are a list");
    assert_eq!(listed[0]["name"], json!("the group"), "the group has its name");
    assert_eq!(listed[0]["members"], json!([line]), "and its member");
}

/// The members of a group move **together**.
#[test]
fn the_members_of_a_group_move_together() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let first = a_line(&mut session, block, "the first", (0.0, 0.0), (10.0, 0.0));
    let second = a_line(&mut session, block, "the second", (0.0, 5.0), (10.0, 5.0));
    let outcome = session
        .execute("cad.group.add", &json!({ "name": "the pair", "block": block, "members": [first, second] }))
        .expect("the group is made");
    let group = outcome.data["group"].as_u64().expect("the group has an id");
    session
        .execute("cad.group.move", &json!({ "group": group, "by": { "x": 0.0, "y": 10.0 } }))
        .expect("the group is moved");
    let node = session.document().node(first).expect("the line is in the document");
    assert_eq!(node.data["geometry"]["from"], json!({ "x": 0.0, "y": 10.0 }), "the first line moved");
    let node = session.document().node(second).expect("the line is in the document");
    assert_eq!(
        node.data["geometry"]["from"],
        json!({ "x": 0.0, "y": 15.0 }),
        "and the second one moved with it"
    );
}

/// A group without a **name** is refused.
#[test]
fn a_group_without_a_name_is_refused() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let line = a_line(&mut session, block, "the line", (0.0, 0.0), (10.0, 0.0));
    let outcome =
        session.execute("cad.group.add", &json!({ "name": "  ", "block": block, "members": [line] }));
    assert!(outcome.is_err(), "a group with no name is refused");
    let error = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(error.contains("name"), "and it says why: {error}");
}

/// A group without **members** is refused.
#[test]
fn a_group_without_members_is_refused() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let outcome =
        session.execute("cad.group.add", &json!({ "name": "the group", "block": block, "members": [] }));
    assert!(outcome.is_err(), "a group of nothing is not a group");
    let error = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(error.contains("member"), "and it says why: {error}");
}
