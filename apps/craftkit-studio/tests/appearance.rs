//! The **appearance** in the document: added, changed, and shared.
//!
//! The appearance is the kit's own, and it is shared by **reference**: a
//! thing points at the node that holds the colours, and changing the
//! colours changes everything that points at them.

use craftkit_sdk::serde_json::json;

/// A session over the kit.
fn session() -> craftkit_sdk::Session {
    craftkit_studio::ecosystem().expect("the ecosystem is built").session()
}

/// An **appearance** is a node, and the colours are the kit's own.
#[test]
fn an_appearance_is_a_node() {
    let mut session = session();
    let outcome = session
        .execute("core.appearance.add", &json!({ "name": "ink", "fill": "#3366cc" }))
        .expect("the appearance is added");
    let id = outcome.data.get("id").and_then(|id| id.as_u64()).expect("there is an id");

    let node = session.document().node(id).expect("the node is there");
    assert_eq!(node.kind, "core.appearance");
    let appearance = node.appearance.clone().expect("it has an appearance");
    assert_eq!(
        appearance.fill,
        craftkit_sdk::Fill::Solid(craftkit_sdk::Colour::from_hex("#3366cc").expect("a colour"))
    );
    assert_eq!(appearance.stroke, None, "and no outline");
}

/// A colour that is **not a colour** is refused.
#[test]
fn a_colour_that_is_not_a_colour_is_refused() {
    let mut session = session();
    let error = session
        .execute("core.appearance.add", &json!({ "name": "ink", "fill": "blue" }))
        .expect_err("that is not a colour");
    assert!(error.to_string().contains("rrggbb"), "{error}");
}

/// Two things can **share** one appearance.
///
/// The second thing points at the first one's node, and the colours are
/// changed in one place — which is the whole point of a reference.
#[cfg(feature = "vector")]
#[test]
fn two_things_can_share_one_appearance() {
    let mut session = session();
    let appearance = session
        .execute("core.appearance.add", &json!({ "name": "ink", "fill": "#3366cc" }))
        .expect("the appearance is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let first = session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the first rectangle is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let second = session
        .execute("vector.rectangle", &json!({ "x": 20.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the second rectangle is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");

    for node in [first, second] {
        session
            .execute("node.appearance", &json!({ "node": node, "appearance": appearance }))
            .expect("the node uses the appearance");
    }

    // The document resolves both to the same colours.
    for node in [first, second] {
        let found = session.document().node(node).expect("the node is there");
        assert_eq!(found.appearance_from, Some(appearance), "the node points at the appearance");
        let resolved = session.document().appearance_of(found).expect("the appearance is resolved");
        assert_eq!(
            resolved.fill,
            craftkit_sdk::Fill::Solid(craftkit_sdk::Colour::from_hex("#3366cc").expect("a colour"))
        );
    }

    // And changing the appearance changes both.
    session
        .execute("core.appearance.set", &json!({ "node": appearance, "fill": "#ff0000" }))
        .expect("the appearance is changed");
    for node in [first, second] {
        let found = session.document().node(node).expect("the node is there");
        let resolved = session.document().appearance_of(found).expect("the appearance is resolved");
        assert_eq!(
            resolved.fill,
            craftkit_sdk::Fill::Solid(craftkit_sdk::Colour::from_hex("#ff0000").expect("a colour")),
            "both shapes changed"
        );
    }
}

/// The **path** is drawn in the appearance's colours.
///
/// The paint is given the document, and that is what lets a path follow
/// a reference: a kind that cannot see the document cannot follow one.
#[cfg(feature = "vector")]
#[test]
fn the_path_is_drawn_in_the_appearances_colours() {
    let mut session = session();
    let appearance = session
        .execute("core.appearance.add", &json!({ "name": "ink", "fill": "#3366cc" }))
        .expect("the appearance is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let shape = session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    session
        .execute("node.appearance", &json!({ "node": shape, "appearance": appearance }))
        .expect("the node uses the appearance");

    // The document is drawn into a picture, and the picture carries the
    // colour the appearance named.
    let camera = *session.camera();
    let mut raster = craftkit_raster::Raster::new(64, 64, camera).expect("the picture is made");
    let document = session.document();
    for node in document.walk() {
        let Some(kind) = session.kinds().get(&node.kind) else { continue };
        (kind.paint)(node, document, &mut raster);
    }
    let image = raster.image();
    let painted = (0..image.height())
        .any(|y| (0..image.width()).any(|x| image.pixel(x, y) == Some([0x33, 0x66, 0xcc, 0xff])));
    assert!(painted, "the path was drawn in the appearance's colour");
}
