//! The CAD battery: a drawing is **tables and a flat store**, and the
//! tree is not there.
//!
//! What is proved here: a layer is a table and an entity's layer is an
//! **edge**, a block **contains** its entities while an insert
//! **references** one, the entity kinds are open, and the extents are
//! derived.

use craftkit_sdk::Ecosystem;
#[cfg(feature = "cad")]
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A drawing with a layer and a block.
#[cfg(feature = "cad")]
fn a_drawing() -> craftkit_sdk::Session {
    let mut session = ecosystem().session();
    session.execute("cad.drawing.add", &json!({ "name": "the plan" })).expect("the drawing is added");
    session
        .execute("cad.layer.add", &json!({ "drawing": 1, "name": "Walls", "color": "red" }))
        .expect("the layer is added");
    session.execute("cad.block.add", &json!({ "drawing": 1, "name": "walls" })).expect("the block is added");
    session
}

/// The battery is linked in when the feature is on.
#[cfg(feature = "cad")]
#[test]
fn the_cad_battery_is_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("cad.drawing.add").is_some(), "the battery is in");
    assert!(ecosystem.kinds.get("cad.entity.line").is_some());
    assert!(ecosystem.evaluators.get("cad.extents").is_some());
}

/// And it is not there when the feature is off.
#[cfg(not(feature = "cad"))]
#[test]
fn the_cad_battery_is_not_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("cad.drawing.add").is_none(), "the battery is out");
}

/// The entity kinds are **open**: a line, a circle and a text are three
/// kinds, and nothing limits the list.
#[cfg(feature = "cad")]
#[test]
fn the_entity_kinds_are_open() {
    let ecosystem = ecosystem();
    for kind in
        ["cad.entity.line", "cad.entity.circle", "cad.entity.arc", "cad.entity.text", "cad.entity.insert"]
    {
        assert!(ecosystem.kinds.get(kind).is_some(), "{kind} is a kind");
    }
}

/// An entity's **layer** is an edge, and not a parent.
#[cfg(feature = "cad")]
#[test]
fn an_entitys_layer_is_an_edge() {
    let mut session = a_drawing();
    session
        .execute(
            "cad.entity.line.add",
            &json!({ "block": 3, "name": "the wall", "from": { "x": 0, "y": 0 }, "to": { "x": 10, "y": 0 } }),
        )
        .expect("the line is added");
    session.execute("cad.entity.layer", &json!({ "entity": 4, "layer": 2 })).expect("it is on the layer");
    let relations = &session.document().relations;
    // The edge is the **kit's** edge: every battery that has layers
    // uses it, so that the rule that follows from it is one rule.
    let edge = relations
        .iter()
        .find(|relation| relation.kind == craftkit_sdk::layer::ON_LAYER)
        .expect("there is an edge");
    assert_eq!(edge.from, 4, "the edge is from the entity");
    assert_eq!(edge.to, 2, "the edge is to the layer");
    assert_eq!(edge.payload["layer"], json!("Walls"), "and the edge says which layer");
    // The entity is still in the block: the layer did not take it over.
    let block = session.document().node(3).expect("the block is there");
    assert_eq!(block.children.len(), 1, "the entity is in the block");
}

/// A block **contains** its entities, and an insert **references** one.
#[cfg(feature = "cad")]
#[test]
fn a_block_contains_and_an_insert_references() {
    let mut session = a_drawing();
    session.execute("cad.block.add", &json!({ "drawing": 1, "name": "door" })).expect("the block is added");
    session
        .execute(
            "cad.entity.line.add",
            &json!({ "block": 4, "name": "the jamb", "from": { "x": 0, "y": 0 }, "to": { "x": 0, "y": 2 } }),
        )
        .expect("the line is added");
    session
        .execute(
            "cad.entity.insert.add",
            &json!({ "block": 3, "name": "the door", "inserts": 4, "at": { "x": 5, "y": 5 } }),
        )
        .expect("the insert is added");
    // The instance and its definition are two things, and the edge
    // between them says which.
    let edge = session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == "cad.inserts")
        .expect("there is an edge");
    assert_eq!(edge.from, 6, "the instance");
    assert_eq!(edge.to, 4, "the definition");
    assert_eq!(edge.payload["block"], json!("door"));
}

/// The **extents** are derived: they are worked out from the geometry
/// every time.
#[cfg(feature = "cad")]
#[test]
fn the_extents_are_derived() {
    let mut session = a_drawing();
    session
        .execute(
            "cad.entity.line.add",
            &json!({ "block": 3, "name": "the wall", "from": { "x": 0, "y": 0 }, "to": { "x": 10, "y": 4 } }),
        )
        .expect("the line is added");
    let plan = session.evaluate();
    let extents: Vec<_> = plan.items.iter().filter(|item| item.kind == "cad.extents").collect();
    assert_eq!(extents[0].value["min"]["x"], json!(0.0));
    assert_eq!(extents[0].value["max"]["x"], json!(10.0));
    assert_eq!(extents[0].value["max"]["y"], json!(4.0));
}

/// And they follow the geometry when it moves.
#[cfg(feature = "cad")]
#[test]
fn the_extents_follow_the_geometry() {
    let mut session = a_drawing();
    session
        .execute(
            "cad.entity.circle.add",
            &json!({ "block": 3, "name": "the tank", "center": { "x": 10, "y": 10 }, "radius": 2 }),
        )
        .expect("the circle is added");
    let plan = session.evaluate();
    let extents: Vec<_> = plan.items.iter().filter(|item| item.kind == "cad.extents").collect();
    assert_eq!(extents[0].value["min"]["x"], json!(8.0), "the circle's box");
    assert_eq!(extents[0].value["max"]["x"], json!(12.0));
}

/// A point that is not a point is refused.
#[cfg(feature = "cad")]
#[test]
fn a_point_that_is_not_a_point_is_refused() {
    let mut session = a_drawing();
    let error = session
        .execute(
            "cad.entity.line.add",
            &json!({ "block": 3, "name": "the wall", "from": { "x": "here", "y": 0 }, "to": { "x": 1, "y": 1 } }),
        )
        .expect_err("the point is not a point");
    assert!(error.to_string().contains("point in the world's units"), "{error}");
}

/// A radius that is not a radius is refused.
#[cfg(feature = "cad")]
#[test]
fn a_radius_that_is_not_a_radius_is_refused() {
    let mut session = a_drawing();
    let error = session
        .execute(
            "cad.entity.circle.add",
            &json!({ "block": 3, "name": "the tank", "center": { "x": 0, "y": 0 }, "radius": 0 }),
        )
        .expect_err("the radius is not a radius");
    assert!(error.to_string().contains("above zero"), "{error}");
}

/// An entity that is put on a layer twice is refused: the edge would be
/// said twice.
#[cfg(feature = "cad")]
#[test]
fn an_entity_on_a_layer_twice_is_refused() {
    let mut session = a_drawing();
    session
        .execute(
            "cad.entity.line.add",
            &json!({ "block": 3, "name": "the wall", "from": { "x": 0, "y": 0 }, "to": { "x": 10, "y": 0 } }),
        )
        .expect("the line is added");
    session.execute("cad.entity.layer", &json!({ "entity": 4, "layer": 2 })).expect("it is on the layer");
    let error = session
        .execute("cad.entity.layer", &json!({ "entity": 4, "layer": 2 }))
        .expect_err("it is already on the layer");
    assert!(error.to_string().contains("already on the layer"), "{error}");
}

/// An insert of something that is not a block is refused.
#[cfg(feature = "cad")]
#[test]
fn an_insert_of_something_that_is_not_a_block_is_refused() {
    let mut session = a_drawing();
    let error = session
        .execute(
            "cad.entity.insert.add",
            &json!({ "block": 3, "name": "the door", "inserts": 2, "at": { "x": 0, "y": 0 } }),
        )
        .expect_err("a layer is not a block");
    assert!(error.to_string().contains("not a block"), "{error}");
}

/// The undo takes the entity and its edge with it.
#[cfg(feature = "cad")]
#[test]
fn the_undo_takes_the_entity_and_its_edge() {
    let mut session = a_drawing();
    session
        .execute(
            "cad.entity.line.add",
            &json!({ "block": 3, "name": "the wall", "from": { "x": 0, "y": 0 }, "to": { "x": 10, "y": 0 } }),
        )
        .expect("the line is added");
    session.execute("cad.entity.layer", &json!({ "entity": 4, "layer": 2 })).expect("it is on the layer");
    assert_eq!(session.document().relations.len(), 1);
    session.undo().expect("the undo works");
    assert_eq!(session.document().relations.len(), 0, "the edge is gone");
    session.undo().expect("the undo works again");
    assert_eq!(session.document().walk().count(), 3, "and so is the entity");
}

/// A line is drawn in **its layer's colour**.
///
/// The entity is *on* a layer, the layer holds an appearance, and the
/// appearance is the kit's own — so the line is drawn in the colour the
/// layer says, and not in a colour it carries itself.
#[cfg(feature = "cad")]
#[test]
fn a_line_is_drawn_in_its_layers_colour() {
    let mut session = a_drawing();
    // The layer says what colour it is.
    let layer = session
        .execute("cad.layer.add", &json!({ "drawing": 1, "name": "blue", "fill": "#3366cc" }))
        .expect("the layer is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let line = session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": 3,
                "name": "a wall",
                "from": { "x": -5.0, "y": -5.0 },
                "to": { "x": 5.0, "y": 5.0 },
            }),
        )
        .expect("the line is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    session
        .execute("cad.entity.layer", &json!({ "entity": line, "layer": layer }))
        .expect("the line is on the layer");

    let picture = draw(&session, 64);
    assert!(picture.contains(&[0x33, 0x66, 0xcc, 0xff]), "the line is in the layer's colour");
}

/// An entity with **no layer** is drawn in black.
///
/// Black is what a drawing is drawn in when it says nothing else.
#[cfg(feature = "cad")]
#[test]
fn an_entity_with_no_layer_is_drawn_in_black() {
    let mut session = a_drawing();
    session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": 3,
                "name": "a wall",
                "from": { "x": -5.0, "y": -5.0 },
                "to": { "x": 5.0, "y": 5.0 },
            }),
        )
        .expect("the line is added");

    let picture = draw(&session, 64);
    assert!(picture.contains(&[0, 0, 0, 0xff]), "the line is in black");
}

/// The picture of a document, as the pixels it has.
#[cfg(feature = "cad")]
fn draw(session: &craftkit_sdk::Session, size: usize) -> Vec<[u8; 4]> {
    let camera = *session.camera();
    let mut raster = craftkit_raster::Raster::new(size, size, camera).expect("the picture is made");
    let document = session.document();
    for node in document.walk() {
        let Some(kind) = session.kinds().get(&node.kind) else { continue };
        (kind.paint)(node, document, &mut raster);
    }
    let image = raster.image();
    let mut pixels = Vec::new();
    for y in 0..image.height() {
        for x in 0..image.width() {
            pixels.push(image.pixel(x, y).unwrap_or([0, 0, 0, 0]));
        }
    }
    pixels
}

/// A line can be **hit**.
///
/// A line has no inside: it is an outline, and a point is on it when
/// it is near it. The window picks with this, so a line that cannot be
/// hit cannot be picked.
#[cfg(feature = "cad")]
#[test]
fn a_line_can_be_hit() {
    let mut session = a_drawing();
    let id = session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": 3,
                "name": "a wall",
                "from": { "x": -5.0, "y": 0.0 },
                "to": { "x": 5.0, "y": 0.0 },
            }),
        )
        .expect("the line is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let document = session.document();
    let line = document.node(id).expect("the line is there");
    let kind = session.kinds().get(&line.kind).expect("the kind is there");

    assert!(
        (kind.hit)(line, document, craftkit_sdk::Point::new(0.0, 0.0)),
        "the middle of the line is on it"
    );
    assert!((kind.hit)(line, document, craftkit_sdk::Point::new(4.9, 0.2)), "and so is a point near it");
    assert!(!(kind.hit)(line, document, craftkit_sdk::Point::new(0.0, 20.0)), "and a point far away is not");
}

/// A circle is hit **on its edge**, not in its middle.
///
/// The middle of a circle is the inside of it, and the inside is not
/// the circle.
#[cfg(feature = "cad")]
#[test]
fn a_circle_is_hit_on_its_edge() {
    let mut session = a_drawing();
    let id = session
        .execute(
            "cad.entity.circle.add",
            &json!({ "block": 3, "name": "a hole", "center": { "x": 0.0, "y": 0.0 }, "radius": 5.0 }),
        )
        .expect("the circle is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let document = session.document();
    let circle = document.node(id).expect("the circle is there");
    let kind = session.kinds().get(&circle.kind).expect("the kind is there");

    assert!((kind.hit)(circle, document, craftkit_sdk::Point::new(5.0, 0.0)), "the edge is on it");
    assert!(!(kind.hit)(circle, document, craftkit_sdk::Point::new(0.0, 0.0)), "and the middle is not");
}

/// An **insert draws the block it inserts**.
///
/// The insert is a **reference**, and a reference that is drawn as
/// nothing is a reference nobody can see. So the block's own entities
/// are drawn, moved by the insertion point.
#[cfg(feature = "cad")]
#[test]
fn an_insert_draws_the_block_it_inserts() {
    let mut session = a_drawing();
    session.execute("cad.block.add", &json!({ "drawing": 1, "name": "door" })).expect("the block is added");
    session
        .execute(
            "cad.entity.line.add",
            &json!({ "block": 4, "name": "the jamb", "from": { "x": -2, "y": 0 }, "to": { "x": 2, "y": 0 } }),
        )
        .expect("the line is added");
    // The insert goes in the other block, at a place of its own.
    session
        .execute(
            "cad.entity.insert.add",
            &json!({ "block": 3, "name": "the door", "inserts": 4, "at": { "x": 0, "y": 10 } }),
        )
        .expect("the insert is added");

    let picture = draw(&session, 64);
    // The line of the block is drawn twice: once where it is, and once
    // where the insert puts it. The one at the top is the insert's.
    let drawn: Vec<usize> = picture
        .iter()
        .enumerate()
        .filter(|(_, pixel)| pixel[3] > 0 && pixel[0] < 40)
        .map(|(index, _)| index)
        .collect();
    assert!(!drawn.is_empty(), "something was drawn");
    let rows: Vec<usize> = drawn.iter().map(|index| index / 64).collect();
    let lowest = rows.iter().min().expect("there are rows");
    let highest = rows.iter().max().expect("there are rows");
    assert!(highest - lowest > 8, "the line is drawn twice, in two places: {lowest} to {highest}");
}

/// An insert is **hit as the block it inserts**.
///
/// The insert is drawn as the block, so it is hit as the block: this is
/// why the hit test is given the document.
#[cfg(feature = "cad")]
#[test]
fn an_insert_is_hit_as_the_block_it_inserts() {
    let mut session = a_drawing();
    session.execute("cad.block.add", &json!({ "drawing": 1, "name": "door" })).expect("the block is added");
    session
        .execute(
            "cad.entity.line.add",
            &json!({ "block": 4, "name": "the jamb", "from": { "x": -2, "y": 0 }, "to": { "x": 2, "y": 0 } }),
        )
        .expect("the line is added");
    session
        .execute(
            "cad.entity.insert.add",
            &json!({ "block": 3, "name": "the door", "inserts": 4, "at": { "x": 0, "y": 10 } }),
        )
        .expect("the insert is added");

    let document = session.document();
    let insert = document.walk().find(|node| node.kind == "cad.entity.insert").expect("there is an insert");
    let kind = session.kinds().get(&insert.kind).expect("the kind is there");
    assert!(
        (kind.hit)(insert, document, craftkit_sdk::Point::new(0.0, 10.0)),
        "the middle of the inserted line is on the insert"
    );
    assert!(
        !(kind.hit)(insert, document, craftkit_sdk::Point::new(0.0, 40.0)),
        "and a point far away is not"
    );
}
