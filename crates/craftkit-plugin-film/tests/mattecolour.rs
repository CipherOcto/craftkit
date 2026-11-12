//! The **colour** of a colour matte: the tree's own rule that a matte
//! may be recoloured, and that an item that is not a matte may not.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the film battery.
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

/// A **bin** to put the items in.
fn a_bin(session: &mut Session) -> u64 {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    outcome.data["id"].as_u64().expect("the bin has an id")
}

/// A **colour matte** in the bin, and its id.
fn a_matte(session: &mut Session, bin: u64) -> u64 {
    let outcome = session.execute(
        "film.item.generator",
        &json!({ "name": "matte", "bin": bin, "item": 7, "kind": "Color Matte" }),
    );
    outcome.expect("the matte is added").data["id"].as_u64().expect("the item has an id")
}

/// The **colour** of a matte, as the document carries it.
fn colour_of(session: &Session, item: u64) -> String {
    let node = session.document().node(item).expect("the item is there");
    node.data["generator"]["colour"].as_str().expect("the colour is there").to_string()
}

/// The colour of a matte may be changed.
#[test]
fn the_colour_of_a_matte_may_be_changed() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let item = a_matte(&mut session, bin);
    assert_eq!(colour_of(&session, item), "#1a1a1a", "the matte is the tree's dark grey");
    session
        .execute("film.item.matte.colour", &json!({ "item": item, "colour": "#ff0000" }))
        .expect("the colour is changed");
    assert_eq!(colour_of(&session, item), "#ff0000", "and the colour is changed");
}

/// An item that is **not** a colour matte may not be recoloured.
#[test]
fn an_item_that_is_not_a_matte_is_refused() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let outcome = session.execute(
        "film.item.generator",
        &json!({ "name": "black", "bin": bin, "item": 8, "kind": "Black Video" }),
    );
    let item = outcome.expect("the generator is added").data["id"].as_u64().expect("the item has an id");
    let error = session
        .execute("film.item.matte.colour", &json!({ "item": item, "colour": "#ff0000" }))
        .expect_err("the item is refused");
    assert!(error.to_string().contains("is not a Color Matte"), "the refusal says so: {error}");
}

/// A colour that is **not** a colour is refused.
#[test]
fn a_colour_that_is_not_a_colour_is_refused() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let item = a_matte(&mut session, bin);
    let error = session
        .execute("film.item.matte.colour", &json!({ "item": item, "colour": "red" }))
        .expect_err("the colour is refused");
    assert!(error.to_string().contains("colour"), "the refusal says so: {error}");
}

/// An item that is **not there** may not be recoloured.
#[test]
fn an_item_that_is_not_there_is_refused() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let _ = bin;
    let error = session
        .execute("film.item.matte.colour", &json!({ "item": 999, "colour": "#ff0000" }))
        .expect_err("the item is refused");
    assert!(error.to_string().contains("there is no item `999`"), "the refusal says so: {error}");
}
